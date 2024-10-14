
// Params that control how we will iterate across the log file

use crate::{indexer::line_indexer::IndexedLog, LineIndexerDataIterator, LogLine};

#[derive(Clone, Copy)]
pub enum LineViewMode{
    Wrap{width: usize},
    Chop{width: usize, left: usize},
    WholeLine,
}

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
            if buffer.line.is_empty() {
                None
            } else {
                assert!(index < buffer.line.len(), "Subline index out of bounds {} >= {}", index, buffer.line.len());
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

    // If we're wrapping lines, this helper splits the initial line into fwd and rev chunks given some desired offset starting point.
    // Two new SubLineHelpers will be returned to be used for the "fwd" and "rev" iterators.
    // The rev helper will be built to return the chunk before the one containing the offset. The fwd will ref the chunk containing the offset.
    fn chop_prev(buffer: LogLine, mode: &LineViewMode, offset: usize) -> (SubLineHelper, SubLineHelper) {
        let mut rev = Self::new();
        rev.init_back(mode, Some(buffer));
        match mode {
            LineViewMode::Wrap{width} => {
                if rev.contains(offset) {
                    // We're definitely going to split the buffer. Determine where and adjust the index.
                    let buffer = rev.buffer.as_ref().unwrap();
                    let fwd_index = (offset - buffer.offset) / width * width;

                    // Construct a SubLineHelper for the fwd iterator
                    let fwd_buf = if fwd_index > 0 {
                        rev.index = fwd_index - width;
                        Some(LogLine::new(buffer.line.clone(), buffer.offset))
                    } else {
                        // Fwd offset is in the first chunk; we don't have any rev chunk remaining
                        rev.buffer.take()
                    };
                    let fwd = Self { index: fwd_index, buffer: fwd_buf };
                    (rev, fwd)
                } else {
                    // TODO assert buffer.offset + buffer.line.len() == offset
                    (rev, Self::new())
                }
            },
            _ => (rev, Self::new()),
        }
    }
}

// Iterate over line subsections as position, offset, string
// This iterator handles breaking lines into substrings for wrapping, right-scrolling, and/or chopping
pub struct SubLineIterator<'a, LOG: IndexedLog> {
    inner: LineIndexerDataIterator<'a, LOG>,
    mode: LineViewMode,
    fwd: SubLineHelper,
    rev: SubLineHelper,

    // Start of first line; splits line between fwd and rev if necessary
    start: Option<usize>,
}

impl<'a, LOG: IndexedLog> SubLineIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG, mode: LineViewMode) -> Self {
        let inner = LineIndexerDataIterator::new(log);
        // TODO: handle rev() getting last subsection of last line somewhere
        Self {
            inner,
            mode,
            fwd: SubLineHelper::new(),
            rev: SubLineHelper::new(),
            start: None,
        }
    }

    pub fn new_from(log: &'a mut LOG, mode: LineViewMode, offset: usize) -> Self {
        let inner = LineIndexerDataIterator::new_from(log, offset);

        Self {
            inner,
            mode,
            fwd: SubLineHelper::new(),
            rev: SubLineHelper::new(),
            start: Some(offset),
        }
    }
}

impl<'a, LOG: IndexedLog>  SubLineIterator<'a, LOG> {
    // Usually when an offset is given we can count on the lineindexer to correctly load the previous line and next line correctly.
    // But if we are wrapping lines, the "next" and "prev" chunks may come from the same line in the file. We handle this here.
    // When we load the first line of this iterator, if an offset was given, we may need to split the line into two chunks.
    // If the offset was at the start of the line, we don't need to do anything.  But if it was in the middle, then the line we
    // need will be in the "prev" line loader.  That is, it will be before the given offset.  So we need to load the previous line
    // and split it in two. The chop_prev function handles cleaving at the right place.
    #[inline]
    fn adjust_first_helpers(&mut self) {
        if let Some(offset) = self.start {
            assert!(self.rev.buffer.is_none());
            assert!(self.fwd.buffer.is_none());
            if let LineViewMode::Wrap{width: _} = self.mode {
                if let Some(prev) = self.inner.next_back() {
                    (self.rev, self.fwd) = SubLineHelper::chop_prev(prev, &self.mode, offset);
                }
            }
            self.start = None;
        }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for SubLineIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.adjust_first_helpers();
        let ret = self.rev.sub_next_back(&self.mode);
        if ret.is_some() {
            ret
        } else {
            self.rev.next_back(&self.mode, self.inner.next_back())
        }
    }
}

impl<'a, LOG: IndexedLog> Iterator for SubLineIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.adjust_first_helpers();
        let ret = self.fwd.sub_next(&self.mode);
        if ret.is_some() {
            ret
        } else {
            self.fwd.next(&self.mode, self.inner.next())
        }
    }
}
