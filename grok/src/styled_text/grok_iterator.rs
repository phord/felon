use std::ops::{Bound, Range};

use indexed_file::{IndexedLog, LineIndexerDataIterator, LogLine};

use super::Stylist;



/// Holds a logline and all of it's styling information before being chopped/wrapped/etc.
/// Supports iterating across the line following a given LineViewMode.
#[derive(Default)]
struct StyledLine {
    line: Option<LogLine>,
    index: usize,
}

impl StyledLine {
    /// Style a new line at a given offset.  Rejects lines whose offset is out of range.
    pub fn new(line: LogLine, offset: usize, stylist: &Stylist) -> Self {
        let index = offset.saturating_sub(line.offset).min(line.line.len().saturating_sub(1));
        if stylist.mode.valid_index(index, line.line.len()) {
            Self {
                line: Some(line),
                index: stylist.mode.chunk_start(index),
                // styles: Vec::new(),
            }
        } else {
            // Index is out of range for this line. Consider us exhausted.
            Self::default()
        }
    }

    fn empty(&self) -> bool {
        self.line.is_none()
    }

    // Get the range of the chunk we're on
    fn chunk_range(&self, stylist: &Stylist) -> Option<Range<usize>> {
        if let Some(line) = &self.line {
            assert!(self.index < line.line.len());
            let end = stylist.mode.chunk_end(self.index, line.line.len());
            Some(self.index..end)
        } else {
            None
        }
    }

    fn advance(&mut self, stylist: &Stylist, forward: bool) -> Option<LogLine> {
        if let Some(range) = self.chunk_range(stylist) {
            let rline = self.render(&range);
            if stylist.mode.is_chunked() {
                let target = if forward { range.end } else { range.start.saturating_sub(1) };
                let next = stylist.mode.chunk_start(target);
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

    fn next(&mut self, stylist: &Stylist) -> Option<LogLine> {
        self.advance(stylist, true)
    }

    fn next_back(&mut self, stylist: &Stylist) -> Option<LogLine> {
        self.advance(stylist, false)
    }

    fn render(&self, range: &Range<usize>) -> LogLine {
        // TODO: construct line with styles
        let line = self.line.as_ref().unwrap();
        let start = range.start;
        let end = range.end.min(line.line.len());
        let rline = line.line[start..end].to_string();
        LogLine::new(rline, line.offset + start)
    }
}


// Iterate over line subsections as position, offset, string
// This iterator handles breaking lines into substrings for wrapping, right-scrolling, and/or chopping
pub struct GrokLineIterator<'a, LOG: IndexedLog> {
    inner: LineIndexerDataIterator<'a, LOG>,
    stylist: &'a Stylist,
    range: Range<usize>,
    fwd: StyledLine,
    rev: StyledLine,
}

impl<'a, LOG: IndexedLog> GrokLineIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG, stylist: &'a Stylist) -> Self {
        let inner = LineIndexerDataIterator::new(log);
        Self {
            inner,
            stylist,
            range: 0..usize::MAX,
            fwd: StyledLine::default(),
            rev: StyledLine::default(),
        }
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
            stylist,
            range: start..end,
            fwd: StyledLine::default(),
            rev: StyledLine::default(),
        }
    }
}

impl<LOG: IndexedLog> DoubleEndedIterator for GrokLineIterator<'_, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.rev.empty() {
            let mut prev = self.inner.next_back();
            if prev.is_none() { prev = self.fwd.line.clone(); }
            if let Some(prev) = prev {
                self.rev = StyledLine::new(prev, self.range.end, self.stylist);
            }
        }
        if let Some(line) = self.rev.next_back(self.stylist) {
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
            let mut next = self.inner.next();
            if next.is_none() { next = self.rev.line.clone(); }
            if let Some(next) = next {
                self.fwd = StyledLine::new(next, self.range.start, self.stylist);
            }
        }
        if let Some(line) = self.fwd.next(self.stylist) {
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
