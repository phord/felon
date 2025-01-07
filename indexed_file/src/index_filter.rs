use log::trace;
use regex::Regex;
use std::ops::Range;

use crate::{indexer::sane_index::SaneIndex, LogLine};
use crate::indexer::waypoint::Position;

/**
 * Basic EventualIndex that accumulates matching line offsets. Can be used for search or filter, despite the name.
 *
 * self.index grows as we navigate around, but it only accumulates lines that match our SearchType. Thus this filter
 * eventually indexes all lines that match the search criteria.
 */

 #[derive(Debug)]
pub enum SearchType {
    Regex(Regex),
    Raw(String),
    Bookmark,
    None,
}

impl std::fmt::Display for SearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchType::Regex(re) => write!(f, "Regex({})", re),
            SearchType::Raw(s) => write!(f, "Raw({})", s),
            SearchType::Bookmark => write!(f, "Bookmark"),
            SearchType::None => write!(f, "None"),
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
        SearchType::Raw(s) => line.contains(s),
        SearchType::None => true,
        _ => { todo!("Unimplemented search type"); false},
    }
}

// Standalone helpers
fn trim_newline(line: &str) -> &str {
    // FIXME: Also remove \r?
    line.strip_suffix("\n").unwrap_or(line)
}

impl Default for IndexFilter {
    fn default() -> Self {
        Self::new(SearchType::None, true)
    }
}

impl IndexFilter {
    pub fn new(f: SearchType, include: bool) -> Self {
        let name = format!("{}", f);
        IndexFilter {
            f,
            include,
            index: SaneIndex::new(name),
        }
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
