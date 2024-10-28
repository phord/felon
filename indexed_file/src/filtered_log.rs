use regex::Regex;

use crate::{index_filter::{IndexFilter, SearchType}, indexer::{eventual_index::{GapRange, Location, TargetOffset, VirtualLocation}, line_indexer::{IndexedLog, LogLocation}}, LogLine};


pub struct FilteredLog<LOG> {
    filter: IndexFilter,
    log: LOG,
}

impl<LOG: IndexedLog> FilteredLog<LOG> {
    pub fn new(log: LOG) -> Self {
        Self {
            filter: IndexFilter::new(SearchType::None),
            log,
        }
    }

    /// Apply a new search to the filter
    /// Invalidates old results
    pub fn search(&mut self, search: SearchType) {
        // TODO: if search != self.filter.f {
        self.filter = IndexFilter::new(search);
    }

    /// Apply a new regex search expression to the filter
    /// Invalidates old results
    pub fn search_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        if re.is_empty() {
            self.search(SearchType::None);
        } else {
            self.search(SearchType::Regex(Regex::new(re)?));
        }
        Ok(())
    }

    // We have a gap in the index. One of the following is true:
    //  The log has no lines between here and the next gap
    //  The log has at least one line covering this location
    // We must resolve the gap in the log if it exists. Then our pos will resolve to a non-gap.
    fn index_chunk(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        use Location::*;
        assert!(pos.tracker.is_gap());
        let seek = pos.tracker.gap_to_target();
        let offset = seek.offset();

        let mut cursor = LogLocation { range: offset..offset, tracker: Virtual(seek) };
        loop {
            if let Some(line) = self.log.next(&mut cursor) {
                pos.tracker = self.filter.eval(&pos.tracker, &cursor.range, &line);
                if !pos.tracker.is_gap() {
                    return Some(line);
                }
            } else {
                // End of file
                pos.tracker = Location::Invalid;
                return None;
            }
        }
    }

    // fill in any gaps by parsing data from the file when needed
    fn resolve_location_filtered(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        // Resolve the location in our filtered index, first. If it's still a gap, we need to resolve it by reading
        // the log and applying the filter there until we get a hit.  This could take a while.
        // Does this need to be cancellable?

        pos.tracker = self.filter.resolve(pos.tracker, self.log.len());
        // TODO: Make callers accept a gap return value. They can handle it by passing a CheckPoint up for the iterator response.
        // Then only try once to resolve the gaps here.

        // Resolve gaps
        while pos.tracker.is_gap() {
            let ret = self.index_chunk(pos);
            pos.tracker = self.filter.resolve(pos.tracker, self.log.len());
            if ret.is_some() {
                pos.tracker = self.filter.next(pos.tracker);
                return ret;
            }
        }

        // We found a region we've seen before.  Read the log line and return it.
        let next = self.filter.next(pos.tracker);
        self.log.read_line(pos, next)
        // FIXME: Could it be an empty index?  In that case, we'll return None here.  Will that confuse the caller into thinking this is the end?
    }
}

// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    #[inline]
    fn next(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        self.resolve_location_filtered(pos)
    }

    #[inline]
    fn len(&self) -> usize {
        self.log.len()
    }

    fn count_lines(&self) -> usize {
        self.filter.count_lines()
    }

    fn read_line(&mut self, pos: &mut LogLocation, next_pos: Location) -> Option<LogLine> {
        self.log.read_line(pos, next_pos)
    }
}


// TODO: Iterators?