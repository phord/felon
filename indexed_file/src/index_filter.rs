use regex::{Error, Regex};
use std::ops::Range;

use crate::{indexer::sane_index::SaneIndex, LogLine};
use crate::indexer::waypoint::Position;

/**
 * Basic Indexer that accumulates matching line offsets. Can be used for search or filter, despite the name.
 *
 * self.index grows as we navigate around, but it only accumulates lines that match our SearchType. Thus this filter
 * eventually indexes all lines that match the search criteria.
 */

 #[derive(Debug)]
pub enum SearchType {
    Regex(Regex),
    Neg(Regex),
    Raw(String),
    None,
}

impl std::fmt::Display for SearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchType::Regex(re) => write!(f, "\"{}\"", re),
            SearchType::Neg(re) => write!(f, "\"!{}\"", re),
            SearchType::Raw(s) => write!(f, "Raw({})", s),
            SearchType::None => write!(f, "None"),
        }
    }
}

impl SearchType {
    pub fn new(s: &str) -> core::result::Result<Self, Error> {
        if s.is_empty() {
            Ok(SearchType::None)
        } else if let Some(stripped) = s.strip_prefix("!") {
            let re = Regex::new(stripped)?;
            Ok(SearchType::Neg(re))
        } else {
            let re = Regex::new(s)?;
            Ok(SearchType::Regex(re))
        }
    }
}

pub struct IndexFilter {
    f: SearchType,

    /// Filter in (true) or out (false)
    include: bool,

    /// Memoized index of matching lines
    pub(crate) index: SaneIndex,
}

#[inline]
fn is_match_type(line: &str, typ: &SearchType) -> bool {
    match typ {
        SearchType::Regex(re) => re.is_match(line),
        SearchType::Neg(re) => !re.is_match(line),
        SearchType::Raw(s) => line.contains(s),
        SearchType::None => true,
    }
}

// Standalone helpers
fn trim_newline(line: &str) -> &str {
    // FIXME: Also remove \r?
    line.strip_suffix("\n").unwrap_or(line)
}

impl Default for IndexFilter {
    fn default() -> Self {
        Self::new(SearchType::None, 0, true)
    }
}

impl IndexFilter {
    pub fn new(f: SearchType, bytes_total: usize, include: bool) -> Self {
        let name = format!("{}", f);
        IndexFilter {
            f,
            include,
            index: SaneIndex::new(name, bytes_total),
        }
    }

    pub fn reset(&mut self) {
        self.index.reset()
    }

    #[inline]
    fn is_match(&self, line: &str) -> bool {
        is_match_type(line, &self.f) ^ (!self.include)
    }

    // Evaluate a new line for inclusion in the index
    pub fn eval(&mut self, line: &LogLine) -> bool {
        self.is_match(trim_newline(line.line.as_str()))
    }

    // Resolve the gap at Position by inserting a new waypoint at the range given
    // Returns the Position of the inserted line
    pub fn insert(&mut self, pos: &Position, range: &Range<usize>) -> Position {
        assert!(pos.is_unmapped());
        self.index.insert_one(pos, range)
    }

    /// Erase the gap at the given position and range.
    /// Returns the position of the next waypoint
    pub fn erase(&mut self, pos: &Position, range: &Range<usize>) -> Position {
        assert!(pos.is_unmapped());
        self.index.erase_gap(pos, range)
    }

    /// Erase the gap at the given position and range.
    /// Returns the position of the previous waypoint, or invalid if there are no earlier ones
    pub fn erase_back(&mut self, pos: &Position, range: &Range<usize>) -> Position {
        assert!(pos.is_unmapped());
        let mut next = self.index.erase_gap(pos, range);
        // erase_gap() may give us the next position which is not what we want; step back one to get the previous one.
        if next.least_offset() > pos.least_offset() {
            next = next.next_back(&self.index);
            assert!(next.is_virtual() || next.least_offset() <= pos.least_offset());
        }
        next
    }

    /// Step to the next indexed line or gap
    #[inline]
    pub fn next(&self, find: &Position) -> Position {
        self.index.next(find)
    }

    /// Step to the prev indexed line or gap
    #[inline]
    pub fn next_back(&self, find: &Position) -> Position {
        self.index.next_back(find)
    }

    /// Resolve Position to an Existing value in the index
    #[inline]
    pub fn resolve(&self, pos: &Position) -> Position {
        pos.resolve(&self.index)
    }

    /// Resolve Position to an Existing value in the index going backwards
    #[inline]
    pub fn resolve_back(&self, pos: &Position) -> Position {
        pos.resolve_back(&self.index)
    }
}
