// A rules engine for applying styles to log lines.
//
// - ANSI normalization / filtering
// - Regex color markup with custom styles
// - Text modification / snipping


use indexed_file::IndexedLog;

use super::GrokLineIterator;

/// Support different line clipping modes:
///  - Chop: break lines at exactly the last byte that fits on the line; show remainder on next line
///  - Clip: clip the leading and trailing portions of the line; do not show remainder
///  - WholeLine: show the whole line; assumes the display will handle wrapping somehow
///  - Wrap: (TODO) wrap text at work breaks
///
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
