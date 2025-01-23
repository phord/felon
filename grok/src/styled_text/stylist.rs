// A rules engine for applying styles to log lines.
//
// - ANSI normalization / filtering
// - Regex color markup with custom styles
// - Text modification / snipping


use crossterm::style::Color;
use indexed_file::IndexedLog;
use regex::Regex;

use super::{styled_line::{PattColor, StyledLine}, GrokLineIterator, LineViewMode};

pub struct Stylist {
    pub mode: LineViewMode,
    // Map of regex -> color pattern
    // TODO: Use PattColor::Plain for uncolored text;  PattColor::NoCrumb for colored output.
    pub patt: PattColor,
    pub styles: Vec<Style>,
}

impl Stylist {
    pub fn new(mode: LineViewMode, patt: PattColor) -> Self {
        Self {
            mode,
            patt,
            styles: Vec::new(),
        }
    }

    pub fn add_style(&mut self, regex: Regex, pattern: PattColor) {
        self.styles.push(Style{matcher: regex, pattern});
    }

    pub fn iter_range<'a, R, T>(&'a self, log: &'a mut T, range: &'a R) -> GrokLineIterator<'a, T>
    where R: std::ops::RangeBounds<usize>, T: IndexedLog
    {
        GrokLineIterator::range(log, self, range)
    }

    pub fn apply(&self, line: &str) -> StyledLine {
        let mut styled = StyledLine::sanitize_basic(line, self.patt);

        // TODO: replace all NoCrumb styles with a Crumb style if one is later matched

        for style in &self.styles {
            for m in style.matcher.captures_iter(line) {
                let m = m.get(0).unwrap();
                let start = m.start();
                let end = m.end();
                // TODO: Custom actions / colors based on style rules
                styled.push(start, end, style.pattern);
            }
        }

        styled
    }
}

/// Transformation rules to apply to a named match
/// - Matcher (regex, string, timestamp, position?)
/// - Action (color, replace, delete, insert, etc.)
/// - Categorize (pid, module, timestamp, etc.)
pub enum StyleAction {
    Basic,              // Match categories and follow user config
    Sanitize,           // Sanitize unprintable control characters a-la less
    Replace(String),    // Replace string with another string
}

pub struct Style {
    pub(crate) matcher: Regex,
    pub(crate) pattern: PattColor,
}
