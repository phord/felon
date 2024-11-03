use log::trace;
use regex::Regex;

use crate::{indexer::eventual_index::{EventualIndex, Location}, LogLine};

/**
 * Basic EventualIndex that accumulates matching line offsets. Can be used for search or filter, despite the name.
 *
 * self.index grows as we navigate around, but it only accumulates lines that match our SearchType. Thus this filter
 * eventually indexes all lines that match the search criteria.
 */

 #[derive(Debug)]
pub enum SearchType {
    Regex(Regex),
    Bookmark,
    None,
}

pub struct IndexFilter {
    f: SearchType,
    index: EventualIndex,
}

#[inline]
fn is_match_type(line: &str, typ: &SearchType) -> bool {
    match typ {
        SearchType::Regex(re) => re.is_match(line),
        SearchType::None => true,
        _ => { todo!("Unimplemented search type"); false},
    }
}

// Standalone helpers
fn trim_newline(line: &str) -> &str {
    // FIXME: Also remove \r?
    line.strip_suffix("\n").unwrap_or(line)
}

impl IndexFilter {
    pub fn new(f: SearchType) -> Self {
        IndexFilter {
            f,
            index: EventualIndex::new(),
        }
    }

    // Evaluate a new line for inclusion in the index
    // returns the next gap or the indexed line, if it matched
    pub fn eval(&mut self, gap: &Location, range: &std::ops::Range<usize>, line: &LogLine) -> Location {
        let found = if is_match_type(trim_newline(line.line.as_str()), &self.f) {
            Some(line.offset)
        } else { None };

        self.index.insert(gap, range, found)
    }

    // Resolve any virtuals into gaps or indexed
    #[inline]
    pub fn resolve(&self, find: Location, end_of_file: usize) -> Location {
        self.index.resolve(find, end_of_file)
    }

    // Step to the next indexed line or gap
    #[inline]
    pub fn next(&self, find: Location) -> Location {
        self.index.next(find)
    }

    #[inline]
    pub fn count_lines(&self) -> usize {
        todo!("self.index.count_lines()");
    }

    // Count the size of the indexed regions
    pub fn indexed_bytes(&self) -> usize {
        self.index.indexed_bytes()
    }

}
