use regex::Regex;

use crate::{log_filter::LogFilter, index_filter::SearchType, indexer::{indexed_log::IndexStats, waypoint::Position, GetLine}, IndexedLog, Log};

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
    /// Invalidates old results
    pub fn search_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        if re.is_empty() {
            self.filter = None;
        } else {
            self.filter = Some(LogFilter::new(SearchType::Regex(Regex::new(re)?)));
        }
        Ok(())
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

    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        if let Some(ref mut filter) = &mut self.filter {
            filter.resolve_gaps(&mut self.source, pos)
        } else {
            self.source.resolve_gaps(pos)
        }
        // TODO: resolve search gaps, too
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
}