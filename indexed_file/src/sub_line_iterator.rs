
// Params that control how we will iterate across the log file

use crate::{indexer::IndexedLog, LineIndexerDataIterator, LogLine};

#[derive(Clone, Copy)]
pub enum LineViewMode{
    Wrap{width: usize},
    Chop{width: usize, left: usize},
    WholeLine,
}

#[derive(Debug)]
struct SubLineHelper {
    // Current line
    buffer: Option<LogLine>,
    // Index into current line for the next chunk to return
    index: usize,
}

impl SubLineHelper {
    fn new() -> Self {
        Self {
            buffer: None,
            index: 0,
        }
    }

    // Returns subbuffer of line with given width if any remains; else None
    fn get_sub(&self, index: usize, width: usize) -> Option<LogLine> {
        if let Some(buffer) = &self.buffer {
            if index >= buffer.line.len() {
                None
            } else {
                let end = (index + width).min(buffer.line.len());
                // Clip the line portion in unicode chars
                let line = buffer.line.chars().take(end).skip(index).collect();
                // FIXME: get printable width by interpreting graphemes? Or punt, because terminals are inconsistent?
                Some(LogLine::new(line, buffer.offset + index))
            }
        } else {
            None
        }
    }

    // Returns next sub-buffer of line if any remains; else None
    fn sub_next(&mut self, mode: &LineViewMode) -> Option<LogLine> {
        match *mode {
            LineViewMode::Wrap{width} => {
                let ret = self.get_sub(self.index, width);
                self.index += width;
                if let Some(buffer) = &self.buffer {
                    if self.index >= buffer.line.len() {
                        // No more to give
                        self.buffer = None;
                    }
                }
                ret
            },
            LineViewMode::Chop{width, left} => {
                let ret = self.get_sub(left, width);
                // No more to give
                self.buffer = None;
                ret
            },
            LineViewMode::WholeLine => {
                self.buffer.take()
            },
        }
    }

    // Returns prev sub-buffer of line if any remains; else None
    fn sub_next_back(&mut self, mode: &LineViewMode) -> Option<LogLine> {
        match *mode {
            LineViewMode::Wrap{width} => {
                let ret = self.get_sub(self.index, width);
                if let Some(buffer) = &self.buffer {
                    if self.index == 0 {
                        // No more to give
                        self.buffer = None;
                    } else if self.index >= width {
                        self.index -= width;
                    } else {
                        // This shouldn't happen, but it can if the width changed between calls.  Prefer not to let that happen.
                        self.buffer = Some(LogLine::new(String::from(&buffer.line[0..self.index]), buffer.offset));
                        self.index = 0;
                        panic!("Subline index underflow. Did width change between calls? width={} index={}", width, self.index);
                    }
                }
                ret
            },
            LineViewMode::Chop{width, left} => {
                let ret = self.get_sub(left, width);
                // No more to give
                self.buffer = None;
                ret
            },
            LineViewMode::WholeLine => {
                self.buffer.take()
            },
        }
    }

    // Supply a new line and get the next chunk
    fn next(&mut self, mode: &LineViewMode, line: Option<LogLine>) -> Option<LogLine> {
        self.buffer = line;
        self.index = 0;
        self.sub_next(mode)
    }

    fn init_back(&mut self, mode: &LineViewMode, line: Option<LogLine>) {
        self.buffer = line;
        if let LineViewMode::Wrap{width} = mode {
            if let Some(buffer) = &self.buffer {
                self.index = if buffer.line.is_empty() {0} else {(buffer.line.len() + width - 1) / width * width - width};
            }
        }
    }

    // Supply a new line and get the last chunk
    fn next_back(&mut self, mode: &LineViewMode, line: Option<LogLine>) -> Option<LogLine> {
        self.init_back(mode, line);
        self.sub_next_back(mode)
    }

    // True if the offset is within the current line
    fn contains(&self, offset: usize) -> bool {
        if let Some(buffer) = &self.buffer {
            offset >= buffer.offset && offset < buffer.offset + buffer.line.len()
        } else {
            false
        }
    }

    // If we're wrapping lines, this helper loads the initial line and finds the sub-offset given some desired starting point.
    // If the offset isn't in this line, it just returns a new helper with no buffer.
    // The helper will be built to return the chunk containing the offset next.
    // This is a non-conforming iterator.  next and next_back move the same index.
    fn chop_prev(buffer: LogLine, mode: &LineViewMode, offset: usize) -> SubLineHelper {
        let mut rev = Self::new();
        rev.init_back(mode, Some(buffer));
        match mode {
            LineViewMode::Wrap{width} => {
                if rev.contains(offset) {
                    // We're definitely going to split the buffer. Determine where and adjust the index.
                    let buffer = rev.buffer.as_ref().unwrap();
                    let fwd_index = (offset - buffer.offset) / width * width;

                    rev.index = fwd_index;
                    rev
                } else {
                    // TODO assert buffer.offset + buffer.line.len() == offset
                    Self::new()
                }
            },
            _ => Self::new(),
        }
    }
}

// Iterate over line subsections as position, offset, string
// This iterator handles breaking lines into substrings for wrapping, right-scrolling, and/or chopping
pub struct SubLineIterator<'a, LOG: IndexedLog> {
    inner: LineIndexerDataIterator<'a, LOG>,
    mode: LineViewMode,
    sub: SubLineHelper,

    // Start of first line; used to load and split the first line if we're in the middle
    start: Option<usize>,
}

impl<'a, LOG: IndexedLog> SubLineIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG, mode: LineViewMode) -> Self {
        let inner = LineIndexerDataIterator::new(log);
        // TODO: handle rev() getting last subsection of last line somewhere
        Self {
            inner,
            mode,
            sub: SubLineHelper::new(),
            start: None,
        }
    }

    pub fn new_from(log: &'a mut LOG, mode: LineViewMode, offset: usize) -> Self {
        let inner = LineIndexerDataIterator::range(log, ..offset);
        todo!("Replace with 'range' function, and fix fwd/rev accordingly; remove 'start' field");

        Self {
            inner,
            mode,
            sub: SubLineHelper::new(),
            start: Some(offset),
        }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for SubLineIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(offset) = self.start {
            assert!(self.sub.buffer.is_none());
            if let LineViewMode::Wrap{width: _} = self.mode {
                if let Some(prev) = self.inner.next_back() {
                    self.sub = SubLineHelper::chop_prev(prev, &self.mode, offset);
                    if ! self.sub.contains(offset) {
                        // Non-conforming iterator: undo the next_back by calling next()
                        self.inner.next();
                    }
                }
            }
            self.start = None;
        }
        let ret = self.sub.sub_next_back(&self.mode);
        if ret.is_some() {
            ret
        } else {
            self.sub.next_back(&self.mode, self.inner.next_back())
        }
    }
}

impl<'a, LOG: IndexedLog> Iterator for SubLineIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(offset) = self.start {
            assert!(self.sub.buffer.is_none());
            if let LineViewMode::Wrap{width: _} = self.mode {
                if let Some(prev) = self.inner.next_back() {
                    self.sub = SubLineHelper::chop_prev(prev, &self.mode, offset);
                    // Non-conforming iterator: undo the next_back by calling next()
                    self.inner.next();
                }
            }
            self.start = None;
        }
        let ret = self.sub.sub_next(&self.mode);
        if ret.is_some() {
            ret
        } else {
            self.sub.next(&self.mode, self.inner.next())
        }
    }
}
