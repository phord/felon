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
        self.search(SearchType::Regex(Regex::new(re)?));
        Ok(())
    }

    // We have a gap in the index. One of the following is true:
    //  The log has no lines between here and the next gap
    //  The log has at least one line covering this location
    // We must resolve the gap in the log if it exists. Then our pos will resolve to a non-gap.
    fn index_chunk(&mut self, gap: Location) -> Location {
        use Location::*;
        assert!(gap.is_gap());
        let seek = gap.gap_to_target();
        let offset = seek.offset();

        let mut cursor = LogLocation { range: offset..offset, tracker: Virtual(seek) };
        while !cursor.tracker.is_invalid() {
            if let Some(line) = self.log.next(&mut cursor) {
                let gap = self.filter.eval(gap, &cursor.range, &line.line, line.offset);
                if !gap.is_gap() {
                    return gap;
                }
            } else {
                break
            }
        }
        if offset < self.log.len() {
            dbg!(offset, self.log.len());
            self.filter.resolve(Virtual(seek), self.log.len())
        } else {
            Location::Invalid
        }
    }

    // fill in any gaps by parsing data from the file when needed
    fn resolve_location_filtered(&mut self, pos: Location) -> Location {
        // Resolve the location in our filtered index, first. If it's still a gap, we need to resolve it by reading
        // the log and applying the filter there until we get a hit.  This could take a while.
        // Does this need to be cancellable?

        let mut pos = self.filter.resolve(pos, self.log.len());
        // TODO: Make callers accept a gap return value. They can handle it by passing a CheckPoint up for the iterator response.
        // Then only try once to resolve the gaps here.

        // Resolve gaps
        while pos.is_gap() {
            pos = self.index_chunk(pos);
            pos = self.filter.resolve(pos, self.log.len());
        }
        pos
    }
}

// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    #[inline]
    fn next(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        // %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
        // FIXME: Figure out how to reimplement this in terms of IndexedLog::next
        // FIXME: Get rid of read_line and use log.next instead
        // %%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%
        pos.tracker = self.resolve_location_filtered(pos.tracker);
        let next = self.filter.next(pos.tracker);
        self.log.read_line(pos, next)
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