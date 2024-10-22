use crate::{index_filter::{IndexFilter, SearchType}, indexer::{eventual_index::{Location, VirtualLocation}, line_indexer::IndexedLog}};


pub struct FilteredLog {
    filter: IndexFilter,
    log: Box<dyn IndexedLog>,
}

impl FilteredLog {
    pub fn new(log: Box<dyn IndexedLog>) -> Self {
        Self {
            filter: IndexFilter::new(SearchType::None),
            log,
        }
    }

    pub fn search(&mut self, search: SearchType) {
        // TODO: if search != self.filter.f {
        self.filter = IndexFilter::new(search);
    }

    // We have a gap in the index. One of the following is true:
    //  The log has no lines between here and the next gap
    //  The log has at least one line covering this location
    // We must resolve the gap in the log if it exists. Then our pos will resolve to a non-gap.
    fn index_chunk(&mut self, gap: Location) -> Location {
        use Location::*;
        use VirtualLocation::*;
        // dbg!(gap);
        if gap.is_gap() {
            let seek = gap.make_portable();
            // dbg!(seek);
            let pos = self.log.resolve_location(seek);
            assert!(!pos.is_gap(), "resolve_location should not return a gap, right?");
            // dbg!(pos);
            assert!(pos.is_indexed());

            if !pos.is_gap() {
                // We found a line using our gap. Resolve the range now covered using both.

                if let Some(offset) = pos.offset() {

                    match self.log.read_line_at(offset) {
                        Ok(line) => {
                            let (start, end) = match seek {
                                Virtual(Before(_)) => (offset, offset + line.len()),
                                Virtual(AtOrAfter(off)) => (off, offset + line.len()),
                                _ => panic!("Unexpected virtual seek type: {:?}", seek),
                            };
                            assert!(start <= offset);
                            assert!(end >= offset);
                            assert!(end <= self.log.len());
                            // dbg!(start);
                            // dbg!(end);

                            let range = std::ops::Range {start, end};

                            self.filter.eval(gap, range, &line, offset);

                            if start >= end {
                                // End of file
                                return Location::Invalid
                            }
                        },
                        Err(e) => {
                            panic!("Error reading line at offset {}: {}", offset, e);
                        },
                    }
                } else {
                    // dbg!(pos);
                    panic!("Not a gap but no offset, either?");
                }
            }
            // dbg!(pos);
            self.filter.resolve(pos.make_portable(), self.log.len())
        } else {
            gap
        }
    }
}

// Navigation
impl IndexedLog for FilteredLog {
    #[inline]
    // fill in any gaps by parsing data from the file when needed
    fn resolve_location(&mut self, pos: Location) -> Location {
        // Resolve the location in our filtered index, first. If it's still a gap, we need to resolve it by reading
        // the log and applying the filter there until we get a hit.  This could take a while.
        // Does this need to be cancellable?

        let mut pos = self.filter.resolve(pos, self.log.len());
        // TODO: Make callers accept a gap return value. They can handle it by passing a CheckPoint up for the iterator response.
        // Then only try once to resolve the gaps here.
        // dbg!(pos);

        // let mut i = 0;
        // Resolve gaps
        while pos.is_gap() {
            pos = self.index_chunk(pos);
            // dbg!(pos);
            pos = self.filter.resolve(pos, self.log.len());
            // dbg!(pos);
            // i += 1;
            // assert!(i < 50);
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