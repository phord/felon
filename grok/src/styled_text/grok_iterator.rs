use std::ops::{Bound, Range};

use indexed_file::{IndexedLog, LineIndexerDataIterator, LogLine};

use super::{styled_line::StyledLine, Stylist};



/// Holds a logline and all of it's styling information before being chopped/wrapped/etc.
/// Supports iterating across the line following a given Stylist's LineViewMode.
struct SubLineHelper<'a> {
    line: Option<StyledLine>,
    offset: usize,
    index: usize,
    stylist: &'a Stylist,
}

impl<'a> SubLineHelper<'a> {
    /// Style a new line at a given offset.  Rejects lines whose offset is out of range.
    fn new(stylist: &'a Stylist) -> Self {
        Self {
            line: None,
            offset: 0,
            index: 0,
            stylist,
        }
    }

    /// Accept a new line and position to begin iterating
    fn insert(&mut self, line: LogLine, offset: usize) {
        self.offset = line.offset;
        let line = self.stylist.apply(&line.line);
        let index = offset.saturating_sub(self.offset).min(line.line.len().saturating_sub(1));
        self.index = self.stylist.mode.chunk_start(index);
        self.line = Some(line);
    }

    // Copy an existing SubLineHelper but use a different index
    fn from(&mut self, other: &Self, index: usize) {
        self.line = other.line.clone();
        self.offset = other.offset;
        self.index = index;
    }

    fn empty(&self) -> bool {
        self.line.is_none()
    }

    // Get the range of the chunk we're on
    fn chunk_range(&self) -> Option<Range<usize>> {
        if let Some(line) = &self.line {
            assert!(self.index < line.line.len() || line.line.is_empty());
            let end = self.stylist.mode.chunk_end(self.index, line.line.len());
            Some(self.index..end)
        } else {
            None
        }
    }

    fn advance(&mut self, forward: bool) -> Option<LogLine> {
        if let Some(range) = self.chunk_range() {
            let rline = self.render(&range);
            if self.stylist.mode.is_chunked() {
                let target = if forward { range.end } else { range.start.saturating_sub(1) };
                let next = self.stylist.mode.chunk_start(target);
                // If there is a valid next chunk, it start be outside the range of this one but still within the line
                if !range.contains(&next) && next < self.line.as_ref().unwrap().line.len() {
                    self.index = next;
                } else {
                    // No more chunks
                    self.line = None;
                }
            } else {
                // No more chunks
                self.line = None;
            }
            Some(rline)
        } else {
            None
        }
    }

    fn next(&mut self) -> Option<LogLine> {
        self.advance( true)
    }

    fn next_back(&mut self) -> Option<LogLine> {
        self.advance( false)
    }

    fn render(&self, range: &Range<usize>) -> LogLine {
        let line = self.line.as_ref().unwrap();
        let rline = line.to_string(range.start, range.end - range.start);
        LogLine::new(rline, self.offset + range.start)
    }
}


// Iterate over line subsections as position, offset, string
// This iterator handles breaking lines into substrings for wrapping, right-scrolling, and/or chopping
pub struct GrokLineIterator<'a, LOG: IndexedLog> {
    inner: LineIndexerDataIterator<'a, LOG>,
    range: Range<usize>,
    fwd: SubLineHelper<'a>,
    rev: SubLineHelper<'a>,
}

impl<'a, LOG: IndexedLog> GrokLineIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG, stylist: &'a Stylist) -> Self {
        Self::range(log, stylist, &(..))
    }

    pub fn range<R>(log: &'a mut LOG, stylist: &'a Stylist, range: &'a R) -> Self
    where
        R: std::ops::RangeBounds<usize>,
    {
        let start = start_offset(range.start_bound());
        let end = end_offset(range.end_bound());
        let inner = LineIndexerDataIterator::range(log, range);

        Self {
            inner,
            range: start..end,
            fwd: SubLineHelper::new(stylist),
            rev: SubLineHelper::new(stylist),
        }
    }
}

impl<LOG: IndexedLog> DoubleEndedIterator for GrokLineIterator<'_, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.rev.empty() {
            if let Some(prev) = self.inner.next_back() {
                self.rev.insert(prev, self.range.end);
            } else {
                self.rev.from(&self.fwd, self.range.end);
            }
        }
        if let Some(line) = self.rev.next_back() {
            if line.offset >= self.range.start {
                self.range = self.range.start..line.offset;
                return Some(line);
            }
        }
        None
    }
}

impl<LOG: IndexedLog> Iterator for GrokLineIterator<'_, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.fwd.empty() {
            if let Some(next) = self.inner.next() {
                self.fwd.insert(next, self.range.start);
            } else {
                self.fwd.from(&self.rev, self.range.start);
            }
        }
        if let Some(line) = self.fwd.next() {
            if line.offset < self.range.end {
                self.range = line.offset.saturating_add(1)..self.range.end;
                return Some(line)
            }
        }
        None
    }
}



// TODO: Dedup with iterator.rs:
// returns the byte at the start of our range, inclusive
fn start_offset(bound: Bound<&usize>) -> usize {
    match bound {
        Bound::Included(val) => *val,
        Bound::Excluded(val) => val.saturating_add(1),
        Bound::Unbounded => 0,
    }
}

// End returns the byte after our range, exclusive
fn end_offset(bound: Bound<&usize>) -> usize {
    match bound {
        Bound::Included(val) => val.saturating_add(1),
        Bound::Excluded(val) => *val,
        Bound::Unbounded => usize::MAX,
    }
}
