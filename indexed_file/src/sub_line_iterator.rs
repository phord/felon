
// Params that control how we will iterate across the log file

use std::ops::Bound;

use crate::{indexer::IndexedLog, LineIndexerDataIterator, LogLine};

#[derive(Clone, Copy, Debug)]
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

    // Start offset for the iterator
    start: Option<usize>,

    // Last offset consumed
    consumed: Option<usize>,
}

impl SubLineHelper {
    fn new() -> Self {
        Self {
            buffer: None,
            index: 0,
            start: None,
            consumed: None,
        }
    }

    fn new_from(offset: usize) -> Self {
        Self {
            buffer: None,
            index: 0,
            start: Some(offset),
            consumed: None,
        }
    }

    fn offset(&self) -> usize {
        self.consumed.unwrap()
    }

    fn le(&self, other: &Self) -> bool {
        self.consumed.is_some() && other.consumed.is_some() && self.offset() <= other.offset()
    }

    // Returns subbuffer of line with given width if any remains; else None
    fn get_sub(&self, index: usize, width: usize) -> Option<LogLine> {
        if let Some(buffer) = &self.buffer {
            if index >= buffer.line.len() {
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
                self.mark_consumed();
                if let Some(buffer) = &self.buffer {
                    if self.index >= buffer.line.len() {
                        // No more to give
                        self.buffer = None;
                    }
                }
                ret
            },
            LineViewMode::Chop{width, left} => {
                self.mark_consumed();
                let ret = self.get_sub(left, width);
                // No more to give
                self.buffer = None;
                ret
            },
            LineViewMode::WholeLine => {
                self.mark_consumed();
                self.buffer.take()
            },
        }
    }

    fn mark_consumed(&mut self) {
        if let Some(buffer) = &self.buffer {
            self.consumed = Some(buffer.offset + self.index);
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
                self.mark_consumed();
                ret
            },
            LineViewMode::Chop{width, left} => {
                self.mark_consumed();
                let ret = self.get_sub(left, width);
                // No more to give
                self.buffer = None;
                ret
            },
            LineViewMode::WholeLine => {
                self.mark_consumed();
                self.buffer.take()
            },
        }
    }

    fn init_fwd(&mut self, mode: &LineViewMode, line: Option<LogLine>) {
        self.buffer = line;
        if let LineViewMode::Wrap{width} = mode {
            if let Some(buffer) = &self.buffer {
                if !buffer.line.is_empty() {
                    self.index =
                        if let Some(start) = self.start {
                            if (buffer.offset..buffer.offset + buffer.line.len()).contains(&start) {
                                // position to start of the chunk after the one containing the offset
                                let i = start - buffer.offset;
                                i - i % width
                                // (i + width - 1) / width * width
                            } else {
                                // TODO: dedup this code path
                                // Start is outside this line; presumably before us.
                                // position to start of the first chunk
                                0
                            }
                        } else {
                            // position to start of the first chunk
                            0
                        };
                }
            }
        } else {
            self.index = 0;
        }
    }

    // Supply a new line and get the next chunk
    fn next(&mut self, mode: &LineViewMode, line: Option<LogLine>) -> Option<LogLine> {
        self.init_fwd(mode, line);
        self.sub_next(mode)
    }

    fn init_back(&mut self, mode: &LineViewMode, line: Option<LogLine>) {
        self.buffer = line;
        if let LineViewMode::Wrap{width} = mode {
            if let Some(buffer) = &self.buffer {
                if !buffer.line.is_empty() {
                    self.index =
                        if let Some(start) = self.start {
                            if (buffer.offset..buffer.offset + buffer.line.len()).contains(&start) {
                                // position to start of the chunk containing the offset
                                let i = start - buffer.offset;
                                i - i % width
                            } else {
                                // TODO: dedup this code path
                                // Start is outside this line; Presumably there were no lines before this one.
                                // position to start of the last chunk
                                (buffer.line.len() + width - 1) / width * width - width
                            }
                        } else {
                            // position to start of the last chunk
                            (buffer.line.len() + width - 1) / width * width - width
                        };
                }
            }
        } else {
            self.index = 0;
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
    // Two new SubLineHelpers will be returned to be used for the "fwd" or "rev" iterator.
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
                    let fwd = Self { index: fwd_index, buffer: fwd_buf , start: None, consumed: None};
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
}

// TODO: Dedup this from iterator.rs
fn value_or(bound: Bound<&usize>, def: usize) -> usize {
    match bound {
        Bound::Included(val) => *val,
        Bound::Excluded(val) => val.saturating_sub(1), // FIXME: How to handle ..0?
        Bound::Unbounded => def,
    }
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
        }
    }
    pub fn range<R>(log: &'a mut LOG, mode: LineViewMode, offset: R) -> Self
    where
        R: std::ops::RangeBounds<usize>,
    {
        let fwd = SubLineHelper::new_from(value_or(offset.start_bound(), 0));
        let rev = SubLineHelper::new_from(value_or(offset.end_bound(), usize::MAX));
        let inner = LineIndexerDataIterator::range(log, offset);

        Self {
            inner,
            mode,
            fwd,
            rev,
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
        todo!("Handle case here when both fwd and rev have a start value and both are in the same line");
        // if let Some(offset) = self.start {
        //     assert!(self.rev.buffer.is_none());
        //     assert!(self.fwd.buffer.is_none());
        //     if let LineViewMode::Wrap{width: _} = self.mode {
        //         if let Some(prev) = self.inner.next_back() {
        //             (self.rev, self.fwd) = SubLineHelper::chop_prev(prev, &self.mode, offset);
        //         }
        //     }
        //     self.start = None;
        // }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for SubLineIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        // self.adjust_first_helpers();

        let ret = self.rev.sub_next_back(&self.mode)
            .or_else(|| {self.rev.next_back( &self.mode, self.inner.next_back()) });
        if self.rev.le(&self.fwd) {
            // exhausted
            None
        } else {
            ret
        }
    }
}

impl<'a, LOG: IndexedLog> Iterator for SubLineIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // self.adjust_first_helpers();
        let ret = self.fwd.sub_next(&self.mode)
            .or_else(|| {self.fwd.next( &self.mode, self.inner.next()) });
        if self.rev.le(&self.fwd) {
            // exhausted
            None
        } else {
            ret
        }
    }
}
