use std::time::Duration;

use regex::Regex;

use crate::{index_filter::{IndexFilter, SearchType}, indexer::{timeout::Timeout, GetLine, IndexedLog}, LogLine};

/// Applies an IndexFilter to an IndexedLog to make a filtered IndexLog that can iterate lines after applying the filter.
pub struct FilteredLog<LOG> {
    filter: IndexFilter,
    log: LOG,
    inner_pos: Position,
}

impl<LOG: IndexedLog> FilteredLog<LOG> {
    pub fn new(log: LOG) -> Self {
        Self {
            filter: IndexFilter::default(),
            log,
            inner_pos: Position::invalid(),
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

    /// Find the previous matching line in an unmapped region. Uses inner_pos to track position in log.
    /// Returns the found line and the next-back position from it.
    fn resolve_location_next_back(&mut self, next: &Position) -> GetLine {
        assert!(next.is_unmapped());
        let gap = next.region();
        let mut next = next.clone();

        loop {
            let (pos, line) = self.log.next_back(&self.inner_pos)?;
            self.inner_pos = pos;
            if line.is_none() { break; }
            let line = line.unwrap();
            if line.offset + line.line.len() < gap.start {
                break;
            }
            let range = line.offset..line.offset + line.line.len();
            if self.filter.eval(&line) {
                next = self.filter.insert(&next, &range);
                next = self.filter.next_back(&next);
                return Ok((next, Some(line)));
            } else {
                next = self.filter.erase(&next, &range);
                // erase() may give us the _next_ position which is not what we want; step back one to get the previous one.
                if next.least_offset() > range.start {
                    next = self.filter.next_back(&next);
                    assert!(next.least_offset() <= range.start);
                }
            }
        }

        Ok((next, None))
    }

    // Search an unmapped region for the next line that matches our filter.  Uses inner_pos to track position in log.
    // Returns the found line and the next position from it.
    fn resolve_location_next(&mut self, next: &Position) -> GetLine {
        assert!(next.is_unmapped());
        let gap = next.region();
        let mut next = next.clone();

        if gap.start.max(self.inner_pos.least_offset()) >= gap.end.min(self.log.len()) {
            // EOF: no more lines
            return Ok((Position::invalid(), None));
        }

        loop {
            let (pos, line) = self.log.next(&self.inner_pos)?;
            self.inner_pos = pos;
            if line.is_none() { break; }
            let line = line.unwrap();
            let range = line.offset..line.offset + line.line.len();
            if self.filter.eval(&line) {
                next = self.filter.insert(&next, &range);
                next = self.filter.next(&next);
                return Ok((next, Some(line)));
            } else {
                next = self.filter.erase(&next, &range);
            }
        }

        Ok((next, None))
    }

    // Update an inner Position to navigate the log file while resolving unmapped filtered regions
    fn seek_inner(&mut self, pos: usize) {
        // Ignore it if the caller tries to set us but we're already tracking them
        if self.inner_pos.is_virtual() || !(self.inner_pos.region().contains(&pos) || self.inner_pos.most_offset() == pos) {
            self.inner_pos = Position::from(pos);
        }
    }

    /// Find the next line that matches our filter, memoizing the position in our index.
    fn find_next(&mut self, pos: &Position) -> GetLine {
        let end = self.log.len();

        // Resolve to an existing pos
        // TODO: Do this one time in the iterator constructor
        let offset = pos.least_offset().min(end);
        let mut next = self.filter.resolve(pos);

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() && next.least_offset() < end {
            if next.is_mapped() {
                let offset = next.region().start;
                return Ok((self.filter.next(&next), self.log.read_line(offset)));
            } else if next.is_unmapped() {
                self.seek_inner(offset);
                let (p, line) = self.resolve_location_next(&next)?;
                if line.is_some() {
                    return Ok((p, line));
                }
                next = p;
            } else {
                assert!(next.is_invalid(), "Position should be mapped, unmapped or invalid {:?}", next);
            }
        }
        Ok((next, None))
    }

    /// Find the previous line that matches our filter, memoizing the position in our index.
    fn find_next_back(&mut self, pos: &Position) -> GetLine {

        // TODO: Dedup with find_next:  next_back, resolve_location_next_back are the only differences

        // Resolve to an existing pos
        let offset = pos.most_offset().min(self.log.len().saturating_sub(1));
        let mut next = self.filter.resolve_back(pos);
        if next.least_offset() >= self.log.len() {
            // Force position into valid range
            next = self.filter.next_back(&next);
        }

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() {
            if next.is_mapped() {
                let offset = next.region().start;
                return Ok((self.filter.next_back(&next), self.log.read_line(offset)));
            } else if next.is_unmapped() {
                self.seek_inner(offset);
                let (p, line) = self.resolve_location_next_back(&next)?;
                if line.is_some() {
                    return Ok((p, line));
                }
                if next == p {
                    // Start of file?
                    assert!(next.least_offset() == 0);
                    break;
                }
                next = p;
            } else {
                assert!(next.is_invalid(), "Position should be mapped, unmapped or invalid");
            }
        }
        Ok((next, None))
    }
}

use crate::indexer::waypoint::Position;
// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    #[inline]
    fn next(&mut self, pos: &Position) -> GetLine {
        self.find_next(pos)
    }

    #[inline]
    fn next_back(&mut self, pos: &Position) -> GetLine {
        self.find_next_back(pos)
    }

    #[inline]
    fn len(&self) -> usize {
        self.log.len()
    }

    fn set_timeout(&mut self, limit: Option<Duration>) {
        self.log.set_timeout(limit);
    }

    fn timed_out(&mut self) -> bool {
        self.log.timed_out()
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