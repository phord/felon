// A rules engine for applying styles to log lines.
//
// - ANSI normalization / filtering
// - Regex color markup with custom styles
// - Text modification / snipping

use std::ops::Range;
use std::ops::Bound;

use indexed_file::IndexedLog;
use indexed_file::LineIndexerDataIterator;
use indexed_file::LogLine;

/// Support different line clipping modes:
///  - Chop: break lines at exactly the last byte that fits on the line; show remainder on next line
///  - Clip: clip the leading and trailing portions of the line; do not show remainder
///  - WholeLine: show the whole line; assumes the display will handle wrapping somehow
///  - Wrap: (TODO) wrap text at work breaks
///  TODO: add continuation indent option for chopped/wrapped lines
#[derive(Clone, Copy, Debug)]
pub enum LineViewMode{
    Chop{width: usize},
    Clip{width: usize, left: usize},
    WholeLine,
}

impl LineViewMode {
    /// Return true if the given index is visible in the chopped version of some line with a given length
    /// The pedantic goal is to support range-based iteration where the start/end may not be at line boundaries.
    /// If the part of a line that hits this index is not visible, we can filter the line out of the display.
    /// I'm not sure this is useful.  :-\
    pub fn valid_index(&self, index: usize, len: usize) -> bool {
        match self {
            LineViewMode::Clip{width, left} => index >= *left && index < left + width,
            _ => index < len,
        }
    }

    /// Return the start of the chunk we're on, given an arbitrary offset into the line
    pub fn chunk_start(&self, index: usize) -> usize {
        match self {
            LineViewMode::Chop{width} => index - index % width,
            LineViewMode::Clip{width: _, left} => *left,
            LineViewMode::WholeLine => 0,
        }
    }

    /// Return the end of the chunk we're on, given an arbitrary offset into the line
    pub fn chunk_end(&self, start: usize, end: usize) -> usize {
        match self {
            LineViewMode::Chop{width} => (start + end).min(start + *width),
            LineViewMode::Clip{width, left: _} => (start + end).min(start + *width),
            LineViewMode::WholeLine => end,
        }
    }
}

pub struct Stylist {
    pub mode: LineViewMode,
    // Map of regex -> color pattern
}

impl Stylist {
    pub fn new(mode: LineViewMode) -> Self {
        Self {
            mode,
        }
    }

    pub fn iter_range<'a, R, T>(&'a self, log: &'a mut T, range: &'a R) -> GrokLineIterator<'a, T>
    where R: std::ops::RangeBounds<usize>, T: IndexedLog
    {
        GrokLineIterator::range(log, self, range)
    }
}

/// Holds a logline and all of it's styling information before being chopped/wrapped/etc.
/// Supports iterating across the line following a given LineViewMode.
#[derive(Default)]
pub struct StyledLine {
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

    fn next(&mut self, stylist: &Stylist) -> Option<LogLine> {
        if let Some(range) = self.chunk_range(stylist) {
            let rline = self.render(&range);
            let next = stylist.mode.chunk_start(range.end);
            // If there is a next chunk, it should be same as end of current chunk
            if next == range.end && next < self.line.as_ref().unwrap().line.len() {
                self.index = next;
            } else {
                // No more chunks
                self.line = None;
            }
            Some(rline)
        } else {
            None
        }
    }

    fn next_back(&mut self, stylist: &Stylist) -> Option<LogLine> {
        if let Some(range) = self.chunk_range(stylist) {
            let rline = self.render(&range);
            let next = stylist.mode.chunk_start(range.start.saturating_sub(1));
            // If there is a previous chunk, it should start before us
            if next < range.start {
                self.index = next;
            } else {
                // No more chunks
                self.line = None;
            }
            Some(rline)
        } else {
            None
        }
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