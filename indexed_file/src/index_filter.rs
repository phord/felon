use regex::Regex;

use crate::indexer::eventual_index::{EventualIndex, Location, VirtualLocation};


/**
 * Basic EventualIndex that accumulates matching line offsets. Can be used for search or filter, despite the name.
 */

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
        _ => false,
    }
}

impl IndexFilter {
    pub fn new(f: SearchType) -> Self {
        IndexFilter {
            f,
            index: EventualIndex::new(),
        }
    }

    // Evaluate a line for inclusion in the index if not already seen before
    // Returns true if the line matches the filter
    // TODO: Plumb LogLine through here instead?
    pub fn eval(&mut self, line: &str, offset: usize, eof: usize)  {
        let pos = self.index.resolve(Location::Virtual(VirtualLocation::AtOrAfter(offset)), eof);
        if pos.is_gap() {
            let found = if is_match_type(line, &self.f) {
                Some(offset)
            } else { None };
            let range = std::ops::Range {start: offset, end: offset + line.len() };
            self.index.insert(pos, range, found);
        }
    }
}
