use std::time::Duration;

use regex::Regex;

use crate::{index_filter::{IndexFilter, SearchType}, indexer::{indexed_log::IndexStats, GetLine, IndexedLog}, LogLine};

/// Applies an IndexFilter to an IndexedLog to make a filtered IndexLog that can iterate lines after applying the filter.
struct LogFilter {
    filter: IndexFilter,
    inner_pos: Position,
}

impl LogFilter {
    pub fn new() -> Self {
        Self {
            filter: IndexFilter::default(),
            inner_pos: Position::invalid(),
        }
    }

    /// Apply a new search to the filter
    /// Invalidates old results
    pub fn search(&mut self, search: SearchType, include: bool) {
        // TODO: if search != self.filter.f {
        self.filter = IndexFilter::new(search, include);
    }

    /// Find the previous matching line in an unmapped region. Uses inner_pos to track position in log.
    /// Returns the found line and the next-back position from it.
    fn resolve_location_next_back<LOG: IndexedLog>(&mut self, log: &mut LOG, next: &Position) -> GetLine {
        assert!(next.is_unmapped());
        let gap = next.region();
        let mut next = next.clone();

        loop {
            let get = log.next_back(&self.inner_pos);
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

    // Search an unmapped region for the next line that matches our filter.
    // Uses inner_pos to track position in inner log.
    // Returns the found line and the next position from it.
    fn resolve_location_next<LOG: IndexedLog>(&mut self, log: &mut LOG, next: &Position) -> GetLine {
        assert!(next.is_unmapped());
        let gap = next.region();
        let mut next = next.clone();

        if gap.start.max(self.inner_pos.least_offset()) >= gap.end.min(log.len()) {
            // EOF: no more lines
            return GetLine::Miss(Position::invalid());
        }

        while next.is_unmapped() {
            let gap = next.region();
            let get = log.next(&self.inner_pos);
            if let GetLine::Hit(pos, line) = get {
                self.inner_pos = pos;
                let range = line.offset..line.offset + line.line.len();
                if range.end <= gap.start {
                    // Inner starts by scanning the line that ends at the start of our gap
                    continue;
                }
                assert!(range.start >= gap.start);
                if range.start >= gap.end {
                    // We walked off the end of our gap and onto the next gap.  We're done for now.
                    break;
                }
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
        GetLine::Miss(next)
    }

    // Update an inner Position to navigate the log file while resolving unmapped filtered regions
    fn seek_inner(&mut self, pos: usize) {
        // Ignore it if the caller tries to set us but we're already tracking them
        if self.inner_pos.is_virtual() || !(self.inner_pos.region().contains(&pos) || self.inner_pos.most_offset() == pos) {
            self.inner_pos = Position::from(pos);
        }
    }

    /// Map the next (first) line in an unmapped region, beginning at/after the given offset
    fn explore_unmapped_next<LOG: IndexedLog>(&mut self, log: &mut LOG, pos: &Position, offset: usize) -> GetLine {
        let offset = pos.least_offset().max(offset);
        self.seek_inner(offset);
        self.resolve_location_next(log, pos)
    }

    /// Find the next line that matches our filter, memoizing the position in our index.
    fn find_next<LOG: IndexedLog>(&mut self, log: &mut LOG, pos: &Position) -> GetLine {
        let end = log.len();

        // Resolve to an existing pos
        // TODO: Do this one time in the iterator constructor
        let mut next = self.filter.resolve(pos);

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() && next.least_offset() < end {
            if next.is_mapped() {
                let offset = next.region().start;
                return GetLine::Hit(self.filter.next(&next), log.read_line(offset).unwrap_or_default());
            } else if next.is_unmapped() {
                // Recover the target position from the original Virtual::Offset, or whatever
                let offset = pos.least_offset().min(end);
                let get = self.explore_unmapped_next(log, &next, offset);
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
    fn find_next_back<LOG: IndexedLog>(&mut self, log: &mut LOG, pos: &Position) -> GetLine {

        // TODO: Dedup with find_next:  next_back, resolve_location_next_back are the only differences

        // Resolve to an existing pos
        let mut next = self.filter.resolve_back(pos);
        if next.least_offset() >= log.len() {
            // Force position into valid range
            next = self.filter.next_back(&next);
        }

        // Search until we run off the end, exceed the range, or find a line
        while !next.is_invalid() {
            if next.is_mapped() {
                let offset = next.region().start;
                return GetLine::Hit(self.filter.next_back(&next), log.read_line(offset).unwrap_or_default());
            } else if next.is_unmapped() {
                let offset = pos.most_offset().min(log.len().saturating_sub(1));
                let offset = next.most_offset().min(offset);
                self.seek_inner(offset);
                let get = self.resolve_location_next_back(log, &next);
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


    fn resolve_gaps<LOG: IndexedLog>(&mut self, log: &mut LOG, pos: &Position) -> Position {
        let mut pos = pos.clone();
        while pos.least_offset() < log.len() {
            pos = self.filter.index.seek_gap(&pos);
            while pos.is_unmapped() {
                match self.explore_unmapped_next(log, &pos, 0) {
                    GetLine::Hit(p, _) => pos = p,
                    GetLine::Miss(p) => pos = p,
                    GetLine::Timeout(p) => return p,
                }
            }
        }
        // No more gaps
        Position::invalid()
    }


}

pub struct FilteredLog<LOG> {
    log: LOG,
    filter: LogFilter,
}

impl<LOG: IndexedLog> FilteredLog<LOG> {
    pub fn new(log: LOG) -> Self {
        Self {
            log,
            filter: LogFilter::new(),
        }
    }

    /// Apply a new search to the filter
    /// Invalidates old results
    pub fn search(&mut self, search: SearchType, include: bool) {
        self.filter.search(search, include);
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
}

use crate::indexer::waypoint::Position;
// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        self.filter.resolve_gaps(&mut self.log, pos)
    }

    #[inline]
    fn next(&mut self, pos: &Position) -> GetLine {
        self.filter.find_next(&mut self.log, pos)
    }

    #[inline]
    fn next_back(&mut self, pos: &Position) -> GetLine {
        self.filter.find_next_back(&mut self.log, pos)
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

    fn info(&self) -> impl Iterator<Item = &IndexStats> + '_
    where Self: Sized
    {
        self.log.info().chain(
            std::iter::once(&self.filter.filter.index.stats)
                .filter(|s| s.name != "None")
        )
    }

    fn read_line(&mut self, offset: usize) -> Option<LogLine> {
        self.log.read_line(offset)
    }
}


// TODO: Iterators?