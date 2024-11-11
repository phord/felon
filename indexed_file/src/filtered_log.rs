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

    // We have a gap in the index. Iterate lines from the origin and evaluate each against our filter.
    fn index_chunk(&mut self, range: std::ops::Range<usize>) -> Option<LogLine> {
        use Location::*;

        while !pos.elapsed() {
            let line = self.log.next();
            if line.is_some()  {
                let line = line.unwrap();
                let (loc, matched) = self.filter.eval(&pos.tracker, &cursor.range, &line);
                pos.tracker = loc;
                if matched {
                    return Some(line);
                } else if !pos.tracker.is_gap() {
                    // We finished the gap with no lines found
                    return None;
                }
            } else {
                // End of file
                pos.tracker = Location::Invalid;
                return None;
            }
        }
        None
    }

    // fill in any gaps by parsing data from the file when needed
    fn resolve_location_filtered(&mut self) -> Option<LogLine> {
        // Resolve the location in our filtered index, first. If it's still a gap, we need to resolve it by reading
        // the log and applying the filter there until we get a hit.  This could take a while. LogLocation has a time
        // limit, so we'll yield back if we can't find anything in time.

        todo!("store location internally");
        pos.tracker = self.filter.resolve(pos.tracker, self.log.len());
        // TODO: Let the inner filter return a checkpoint if it's ever going to read from disk twice. We can choose
        // to checkpoint there to limit our forward search in case other work is being done.

        // Resolve gaps
        while pos.tracker.is_gap() {
            let ret = self.index_chunk(pos);
            pos.tracker = self.filter.resolve(pos.tracker, self.log.len());
            if ret.is_some() {
                pos.tracker = self.filter.next(pos.tracker);
                return ret;
            } else if pos.elapsed() {
                return None;
            }
        }

        // We found a region we've seen before.  Read the log line and return it.
        // let next = self.filter.next(pos.tracker);
        let (bytes, line)  = self.log.read_line(0 /*FIXME pos*/);
        // FIXME: move offset by bytes, either fwd or rev
        line
    }
}

// Navigation
impl<LOG: IndexedLog> IndexedLog for FilteredLog<LOG> {
    #[inline]
    fn next(&mut self) -> Option<LogLine> {
        self.resolve_location_filtered()
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

    fn read_line(&mut self, offset: usize) -> (usize, Option<LogLine>) {
        self.log.read_line(offset)
    }
}


// TODO: Iterators?