use regex::Regex;

use crate::{index_filter::{IndexFilter, SearchType}, indexer::{eventual_index::{GapRange, Location, TargetOffset, VirtualLocation}, line_indexer::IndexedLog}};


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

    pub fn search(&mut self, search: SearchType) {
        // TODO: if search != self.filter.f {
        self.filter = IndexFilter::new(search);
    }

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

        // TODO: Dedup the code...
        let it = self.log.iter_lines_from(offset);
        if seek.is_after() {
            // FIXME: is 'offset' really the beginning of our gap?
            let mut start = offset;
            for line in it {
                let end = line.offset + line.line.len();
                let range = std::ops::Range {start, end};
                let gap = self.filter.eval(gap, range, &line.line, line.offset);
                start = end;
                if !gap.is_gap() {
                    return gap;
                }
            }
        } else {
            let mut end = offset;
            for line in it.rev() {
                let start = line.offset;
                let range = std::ops::Range {start, end: end.max(start + line.line.len())};
                let gap = self.filter.eval(gap, range, &line.line, line.offset);
                end = start;
                if !gap.is_gap() {
                    return gap;
                }
            }
        }
        if offset < self.log.len() {
            dbg!(offset, self.log.len());
            self.filter.resolve(Virtual(seek), self.log.len())
        } else {
            // FIXME: Does this work in reverse?
            Location::Invalid
        }
    }
}

// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    #[inline]
    // fill in any gaps by parsing data from the file when needed
    fn resolve_location(&mut self, pos: Location) -> Location {
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

    #[inline]
    fn read_line_at(&mut self, start: usize) -> std::io::Result<String> {
        self.log.read_line_at(start)
    }

    // Step to the next indexed line or gap
    #[inline]
    fn next_line_index(&self, find: Location) -> Location {
        self.filter.next_line_index(find)
    }

    // Step to the previous indexed line or gap
    #[inline]
    fn prev_line_index(&self, find: Location) -> Location {
        self.filter.prev_line_index(find)
    }

    #[inline]
    fn len(&self) -> usize {
        self.log.len()
    }

    fn count_lines(&self) -> usize {
        self.filter.count_lines()
    }
}


// TODO: Iterators?