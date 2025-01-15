use regex::Regex;

use crate::{log_filter::LogFilter, index_filter::SearchType, indexer::{indexed_log::IndexStats, waypoint::Position, GetLine}, IndexedLog, Log};

// TODO: Move this into Grok?  It implements some very grok-specific features.

/// A stack of logs with filters.
/// Rust complicates our traits enough that it's impractical to rely on recursive log trees.
/// As it turns out, that's also impractical from a usability and reasoning standpoint, too.
/// This structure implements our complete stack of logs including the source files, include
/// filters, exclude filters, bookmarks, highlights and and searches.
pub struct LogStack {
    source: Log,
    search: Option<LogFilter>,
    filter: Option<LogFilter>,
}

impl  LogStack {
    pub fn new(log: Log) -> Self {
        Self {
            source: log,
            search: None,
            filter: None,
        }
    }

    /// Apply a new regex search expression to the filter
    /// TODO: add more filters instead of replacing the one we currently allow
    pub fn filter_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        // FIXME: when filter changes, invalidate the search (or merge it / make it dependent on filter)
        if re.is_empty() {
            self.filter = None;
        } else {
            self.filter = Some(LogFilter::new(SearchType::Regex(Regex::new(re)?), self.source.len()));
        }
        Ok(())
    }

    /// Set a new regex search expression
    /// TODO: allow multiple active searches
    pub fn search_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        if re.is_empty() {
            self.search = None;
        } else {
            self.search = Some(LogFilter::new(SearchType::Regex(Regex::new(re)?), self.source.len()));
        }
        Ok(())
    }

    pub fn search_next(&mut self, pos: &Position) -> Position {
        if let Some(ref mut search) = &mut self.search {
            // FIXME: Filter results against self.filter
            search.find_next(&mut self.source, pos).into_pos()
        } else {
            Position::invalid()
        }
    }

    pub fn search_next_back(&mut self, pos: &Position) -> Position {
        if let Some(ref mut search) = &mut self.search {
            // FIXME: Filter results against self.filter
            search.find_next_back(&mut self.source, pos).into_pos()
        } else {
            Position::invalid()
        }
    }

}
impl IndexedLog for LogStack {
    fn read_line(&mut self, offset: usize) -> Option<crate::LogLine> {
        self.source.read_line(offset)
    }

    fn next(&mut self, pos: &Position) -> GetLine {
        if let Some(ref mut filter) = &mut self.filter {
            filter.find_next(&mut self.source, pos)
        } else {
            self.source.next(pos)
        }
    }

    fn next_back(&mut self, pos: &Position) -> GetLine {
        if let Some(ref mut filter) = &mut self.filter {
            filter.find_next_back(&mut self.source, pos)
        } else {
            self.source.next_back(pos)
        }
    }

    fn advance(&mut self, pos: &Position) -> Position {
        if let Some(ref mut filter) = &mut self.filter {
            filter.advance(pos)
        } else {
            self.source.advance(pos)
        }
    }

    fn advance_back(&mut self, pos: &Position) -> Position {
        if let Some(ref mut filter) = &mut self.filter {
            filter.advance_back(pos)
        } else {
            self.source.advance_back(pos)
        }
    }

    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        if let Some(ref mut filter) = &mut self.filter {
            if filter.has_gaps() {
                return filter.resolve_gaps(&mut self.source, pos)
            }
        }

        if let Some(ref mut search) = &mut self.search {
            if search.has_gaps() {
                return search.resolve_gaps(&mut self.source, pos)
            }
        }

        if self.source.has_gaps() {
            return self.source.resolve_gaps(pos)
        }

        Position::invalid()
    }

    fn set_timeout(&mut self, limit: Option<std::time::Duration>) {
        self.source.set_timeout(limit);
    }

    fn timed_out(&mut self) -> bool {
        self.source.timed_out()
    }

    fn len(&self) -> usize {
        self.source.len()
    }

    fn info(&self) -> impl Iterator<Item = &'_ IndexStats> + '_
    where Self: Sized  {
        self.source.info()
        .chain(self.filter.iter().flat_map(|f| f.info()))
        .chain(self.search.iter().flat_map(|f| f.info()))
    }

    fn has_gaps(&self) -> bool {
        self.source.has_gaps() ||
            self.filter.as_ref().map(|f| f.has_gaps()).unwrap_or_else(
                || self.search.as_ref().map(|f| f.has_gaps()).unwrap_or(false)
            )
    }
}