
// Params that control how we will iterate across the log file

use crate::{LineIndexerDataIterator, Log, LogLine};

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

    fn get_sub(&self, index: usize, width: usize) -> Option<LogLine> {
        if let Some(buffer) = &self.buffer {
            assert!(index < buffer.line.len(), "Subline index out of bounds {} >= {}", index, buffer.line.len());
            let end = (index + width).min(buffer.line.len());
            // FIXME: get printable width
            let line = String::from(&buffer.line[index..end]);
            Some(LogLine::new(line, buffer.offset + index))
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

    // Supply a new line and get the last chunk
    fn next_back(&mut self, mode: &LineViewMode, line: Option<LogLine>) -> Option<LogLine> {
        self.buffer = line;
        match mode {
            LineViewMode::Wrap{width} => {
                if let Some(buffer) = &self.buffer {
                    self.index = (buffer.line.len() + width - 1) / width * width - width;
                }
            },
            _ => {},
        }
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
    // This should only be called on the "rev" SubLineHelper; we will modify self to prepare for future calls to sub_next_back.
    // A new SubLineHelper will be returned to be used for the "fwd" iterator, if needed.  If the offset is at the end of the line,
    // or is not contained in this line, or if the mode is not Wrap, then the returned SubLineHelper will be empty.
    // Adjust index to reference the chuck before the one containing the offset
    // Return cloned SubLineHelper with index pointing to fwd chunk that this object (rev) will never use
    fn chop_prev(&mut self, mode: &LineViewMode, offset: usize) -> SubLineHelper {
        match mode {
            LineViewMode::Wrap{width} => {
                if self.contains(offset) {
                    // We're definitely going to split the buffer. Determine where and adjust the index.
                    let buffer = self.buffer.as_ref().unwrap();
                    let fwd_index = (offset - buffer.offset) / width * width;

                    // Construct a SubLineHelper for the fwd iterator
                    let fwd_buf = if fwd_index > 0 {
                        self.index = fwd_index - width;
                        Some(LogLine::new(buffer.line.clone(), buffer.offset))
                    } else {
                        // If the offset is in the first chunk, we don't have any rev chunk remaining
                        self.buffer.take()
                    };
                    Self { index: fwd_index, buffer: fwd_buf }
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
pub struct SubLineIterator<'a> {
    inner: LineIndexerDataIterator<'a>,
    mode: LineViewMode,
    fwd: SubLineHelper,
    rev: SubLineHelper,

    // Start of first line; splits line between fwd and rev if necessary
    start: Option<usize>,
}

impl<'a> SubLineIterator<'a> {
    pub fn new(log: &'a mut Log, mode: LineViewMode) -> Self {
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

    pub fn new_from(log: &'a mut Log, mode: LineViewMode, offset: usize) -> Self {
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

impl<'a>  SubLineIterator<'a> {
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
            self.rev.next_back(&self.mode, self.inner.next());
            self.fwd = self.rev.chop_prev(&self.mode, offset);
            self.start = None;
        }
    }
}

impl<'a> DoubleEndedIterator for SubLineIterator<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.adjust_first_helpers();
        let ret = self.rev.sub_next_back(&self.mode);
        if ret.is_some() {
            ret
        } else {
            self.rev.next_back(&self.mode, self.inner.next())
        }
    }
}

impl<'a> Iterator for SubLineIterator<'a> {
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
