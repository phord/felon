use crate::{index_filter::IndexFilter, indexer::{eventual_index::Location, line_indexer::IndexedLog}};


pub struct FilteredLog {
    filters: Vec<IndexFilter>,
    log: Box<dyn IndexedLog>,
}

impl FilteredLog {
    pub fn new(log: Box<dyn IndexedLog>) -> Self {
        Self {
            filters: Vec::new(),
            log,
        }
    }

    pub fn add_filter(&mut self, filter: IndexFilter) {
        self.filters.push(filter);
    }
}

// Navigation
impl IndexedLog for FilteredLog {
    #[inline]
    fn resolve_location(&mut self, pos: Location) -> Location {
        self.log.resolve_location(pos)
    }

    #[inline]
    fn read_line_at(&mut self, start: usize) -> std::io::Result<String> {
        match  self.log.read_line_at(start) {
            Ok(line) => {
                    for f in &mut self.filters {
                        f.eval(&line, start, self.log.len());
                    }
                    Ok(line)
            },
            Err(e) => Err(e),
        }
    }

    // Step to the next indexed line or gap
    #[inline]
    fn next_line_index(&self, find: Location) -> Location {
        self.log.next_line_index(find)
    }

    // Step to the previous indexed line or gap
    #[inline]
    fn prev_line_index(&self, find: Location) -> Location {
        self.log.prev_line_index(find)
    }

    #[inline]
    fn len(&self) -> usize {
        self.log.len()
    }

    fn count_lines(&self) -> usize {
        self.log.count_lines()
    }
}


// TODO: Iterators?