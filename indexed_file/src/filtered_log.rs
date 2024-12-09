use regex::Regex;

use crate::{index_filter::{IndexFilter, SearchType}, indexer::IndexedLog, LogLine};

/// Applies an IndexFilter to an IndexedLog to make a filtered IndexLog that can iterate lines after applying the filter.
pub struct FilteredLog<LOG> {
    filter: IndexFilter,
    log: LOG,
}

impl<LOG: IndexedLog> FilteredLog<LOG> {
    pub fn new(log: LOG) -> Self {
        Self {
            filter: IndexFilter::default(),
            log,
        }
    }

    /// Apply a new search to the filter
    /// Invalidates old results
    pub fn search(&mut self, search: SearchType, include: bool) {
        // TODO: if search != self.filter.f {
        self.filter = IndexFilter::new(search, include);
    }

    /// Apply a new regex search expression to the filter
    /// Invalidates old results
    pub fn search_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        if re.is_empty() {
            self.search(SearchType::None, true);
        } else {
            self.search(SearchType::Regex(Regex::new(re)?), true);
        }
        Ok(())
    }

    fn resolve_location_next_back(&mut self, next: Position) -> (Position, Option<LogLine>) {
        todo!()
    }

    fn resolve_location_next(&mut self, next: Position) -> (Position, Option<LogLine>) {
        assert!(next.is_unmapped());
        let range = next.region();
        let mut start = range.start;

        let it = self.log.iter_lines_range(start..range.end);
        for line in it {
            let end = range.end.min(line.offset + line.line.len());
            if self.filter.eval(&line) {
                let range = start..end;
                let (next, _prev) = self.filter.insert(next, range, &[line.offset]);
                return (next, Some(line));
            }
            start = end;
        }

        // Didn't find a line in the gap.  Erase the gap and continue.
        let range = start..range.end;
        let (next, _prev) = self.filter.insert(next, range, &[]);
        (next, None)
    }

    fn find_next(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        let end = self.log.len();
        let mut next = pos;

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() && next.least_offset() < end {
            next = self.filter.next(next);
            if next.is_mapped() {
                let offset = next.region().start;
                return (next, self.log.read_line(offset));
            } else if next.is_unmapped() {
                let (p, line) = self.resolve_location_next(next);
                if line.is_some() {
                    return (p, line);
                } // else continue
                next = p;
            } else if next.is_invalid() {
                return (next, None);
            } else {
                panic!("Position should be mapped or unmapped");
            }
        }
        (next, None)
    }
}

use crate::indexer::waypoint::Position;
// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    #[inline]
    // FIXME: next/next_back should take a range.end to search over
    fn next(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        self.find_next(pos)
    }

    #[inline]
    fn next_back(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        self.log.next(pos)
    }

    #[inline]
    fn len(&self) -> usize {
        self.log.len()
    }

    // Count the size of the indexed regions
    fn indexed_bytes(&self) -> usize {
        self.filter.indexed_bytes()
    }

    fn count_lines(&self) -> usize {
        self.filter.count_lines()
    }

    fn read_line(&mut self, offset: usize) -> Option<LogLine> {
        self.log.read_line(offset)
    }
}


// TODO: Iterators?