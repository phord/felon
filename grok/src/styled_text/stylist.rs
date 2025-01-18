// A rules engine for applying styles to log lines.
//
// - ANSI normalization / filtering
// - Regex color markup with custom styles
// - Text modification / snipping


use indexed_file::IndexedLog;

use super::{GrokLineIterator, LineViewMode};

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
