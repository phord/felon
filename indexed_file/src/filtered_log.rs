use std::time::Duration;

use regex::Regex;

use crate::{index_filter::{IndexFilter, SearchType}, indexer::{indexed_log::IndexStats, timeout::Timeout, GetLine, IndexedLog}, LogLine};

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
            let get = self.log.next_back(&self.inner_pos);
            if let GetLine::Hit(pos, line) = get {
                self.inner_pos = pos;
                let range = line.offset..line.offset + line.line.len();
                if line.offset + line.line.len() < gap.start {
                    return GetLine::Miss(next);
                } else if self.filter.eval(&line) {
                    next = self.filter.insert(&next, &range);
                    next = self.filter.next_back(&next);
                    return GetLine::Hit(next, line);
                } else {
                    next = self.filter.erase_back(&next, &range);
                }
            } else {
                return get;
            }
        }
    }

    // Search an unmapped region for the next line that matches our filter.  Uses inner_pos to track position in log.
    // Returns the found line and the next position from it.
    fn resolve_location_next(&mut self, next: &Position) -> GetLine {
        assert!(next.is_unmapped());
        let gap = next.region();
        let mut next = next.clone();

        if gap.start.max(self.inner_pos.least_offset()) >= gap.end.min(self.log.len()) {
            // EOF: no more lines
            return GetLine::Miss(Position::invalid());
        }

        loop {
            let get = self.log.next(&self.inner_pos);
            if let GetLine::Hit(pos, line) = get {
                self.inner_pos = pos;
                let range = line.offset..line.offset + line.line.len();
                if self.filter.eval(&line) {
                    next = self.filter.insert(&next, &range);
                    next = self.filter.next(&next);
                    return GetLine::Hit(next, line);
                } else {
                    next = self.filter.erase(&next, &range);
                }
            } else {
                return get;
            }
        }
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
        let mut next = self.filter.resolve(pos);

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() && next.least_offset() < end {
            if next.is_mapped() {
                let offset = next.region().start;
                return GetLine::Hit(self.filter.next(&next), self.log.read_line(offset).unwrap_or_default());
            } else if next.is_unmapped() {
                // Recover the target position from the original Virtual::Offset, or whatever
                let offset = pos.least_offset().min(end);
                let offset = next.least_offset().max(offset);
                self.seek_inner(offset);
                let get = self.resolve_location_next(&next);
                match get {
                    GetLine::Miss(p) => next = p,  // Resolved gap with no matches; keep searching
                    _ => return get,
                }
            } else {
                assert!(next.is_invalid(), "Position should be mapped, unmapped or invalid {:?}", next);
            }
        }
        GetLine::Miss(next)
    }

    /// Find the previous line that matches our filter, memoizing the position in our index.
    fn find_next_back(&mut self, pos: &Position) -> GetLine {

        // TODO: Dedup with find_next:  next_back, resolve_location_next_back are the only differences

        // Resolve to an existing pos
        let mut next = self.filter.resolve_back(pos);
        if next.least_offset() >= self.log.len() {
            // Force position into valid range
            next = self.filter.next_back(&next);
        }

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() {
            if next.is_mapped() {
                let offset = next.region().start;
                return GetLine::Hit(self.filter.next_back(&next), self.log.read_line(offset).unwrap_or_default());
            } else if next.is_unmapped() {
                let offset = pos.most_offset().min(self.log.len().saturating_sub(1));
                let offset = next.most_offset().min(offset);
                self.seek_inner(offset);
                let get = self.resolve_location_next_back(&next);
                match get {
                    GetLine::Miss(p) => {
                        // Resolved gap with no matches; keep searching unless we hit the start of file
                        if next == p {
                            // Start of file?
                            assert!(next.least_offset() == 0);
                            break;
                        }
                        next = p;
                    },
                    _ => return get,
                }
            } else {
                assert!(next.is_invalid(), "Position should be mapped, unmapped or invalid");
            }
        }
        GetLine::Miss(next)
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

    fn info<'a>(&'a self) -> impl Iterator<Item = &'a IndexStats> + 'a
    where Self: Sized
    {
        self.log.info().chain(
            std::iter::once(&self.filter.index.stats)
                .filter(|s| s.name != "None")
        )
    }

    fn read_line(&mut self, offset: usize) -> Option<LogLine> {
        self.log.read_line(offset)
    }
}


// TODO: Iterators?