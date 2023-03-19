// An index of some lines in a file, possibly with gaps, but eventually a whole index

use crate::index::Index;


pub struct EventualIndex {
    indexes: Vec<Index>,
}

impl EventualIndex {
    pub fn new() -> EventualIndex {
        EventualIndex {
            indexes: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: Index) {
        // merge lazily
        self.indexes.push(other);
    }

    pub fn finalize(&mut self) {
        if self.indexes.is_empty() {
            return;
        }

        self.indexes.sort_by_key(|index| index.start);

        // let mut prev = self.indexes[0].start;
        // for index in self.indexes.iter() {
        //     assert_eq!(index.start, prev);
        //     prev = index.end;
        // }

        // FIXME: Add index for end-of-file if not already present
        // e.g. if self.line_offsets.last() != self.indexes.last().end { self.line_offsets.push(self.indexes.last().end); }

        // FIXME: Merge adjacent indexes if one of them is empty
    }

    // fn line_offset(&self, line_number: usize) -> Option<usize> {
    //     if line_number >= self.lines() {
    //         return None;
    //     }
    //     self.iter().skip(line_number).cloned().next()
    // }

    // fn iter(self: &Self) -> impl DoubleEndedIterator<Item = &usize> {
    //     self.indexes.iter().flat_map(|x| x.iter())
    // }

    #[cfg(test)]
    fn bytes(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.bytes())
    }

    pub fn lines(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.lines())
    }
}

// Delineates [start, end) of a region of the file.  end is not inclusive.
#[derive(Debug, Copy, Clone)]
pub enum Missing {
    // Range has start and end; end is not inclusive
    Bounded(usize, usize),

    // Range has start; end is unknown
    Unbounded(usize),
}

// Literally a reference by subscript to the Index/Line in an EventualIndex.
// Becomes invalid if the EventualIndex changes, but since we use this as a hint only, it's not fatal.
#[derive(Debug, Copy, Clone)]
pub struct IndexRef(usize, usize);

#[derive(Debug, Copy, Clone)]
pub enum VirtualLocation {
    Start,
    End
}

type TargetOffset = usize;

#[derive(Debug, Copy, Clone)]
// Position at `target` is not indexed; need to index region from `gap`
pub struct GapRange {
    pub target: TargetOffset,
    pub gap: Missing,
}
use Missing::{Bounded, Unbounded};

#[derive(Debug, Copy, Clone)]
pub enum Location {
    Virtual(VirtualLocation),
    Indexed(IndexRef),
    Gap(GapRange)
}

// Tests for EventualIndex
#[cfg(test)]
mod tests {
    use crate::index::Index;
    use super::EventualIndex;

    use super::Location;
    use super::GapRange;
    use super::Missing::{Bounded, Unbounded};
    use super::VirtualLocation;
    use super::IndexRef;
    static DATA: &str = "a\na\na\na\na\n";

    fn get_index(offset: usize) -> Index {
        let mut index = Index::new();
        index.parse(DATA.as_bytes(), offset);
        index
    }

    fn get_eventual_index(size: usize) -> EventualIndex {
        let mut index = EventualIndex::new();
        while index.bytes() < size {
            let s = index.bytes();
            println!("Size {s}");
            index.merge(get_index(index.bytes()));
        }
        index.finalize();
        index
    }

    fn get_partial_eventual_index(start: usize, size: usize) -> EventualIndex {
        let mut index = EventualIndex::new();
        while index.bytes() < size {
            let s = index.bytes();
            println!("Size {s}");
            index.merge(get_index(start + index.bytes()));
        }
        index.finalize();
        index
    }

    #[test]
    fn test_eventual_index_basic() {
        let index = get_eventual_index(100);
        assert_eq!(index.bytes(), 100);
        assert_eq!(index.lines(), 50);
    }

    #[test]
    fn test_cursor_start() {
        let index = get_eventual_index(100);
        let cursor = index.locate(0);
        dbg!(cursor);
        match cursor {
            Location::Indexed(IndexRef(0, 0)) => {},
            _ => {
                dbg!(cursor);
                panic!("Expected StartOfFile; got something else");
            }
        }
    }


    #[test]
    fn test_cursor_mid_start() {
        let index = get_partial_eventual_index(50, 100);
        let cursor = index.locate(50);
        match cursor {
            Location::Indexed(IndexRef(0, 0)) => {},
            _ => panic!("Expected Index(0, 0); got something else: {:?}", cursor),
        }
        let fault = index.locate(10);
        match fault {
            Location::Gap(GapRange { gap: Bounded(0, 50), .. } ) => {},
            _ => panic!("Expected Missing(0,50); got something else: {:?}", fault),
        }
    }

    #[test]
    fn test_cursor_last() {
        let index = get_eventual_index(100);
        let cursor = index.locate(index.bytes()-1);
        match cursor {
            Location::Indexed(_) => {},
            _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
        }
        let fault = index.locate(index.bytes());
        match fault {
            Location::Gap(GapRange { gap: Unbounded(_), .. }) => {},
            _ => panic!("Expected MissingUnbounded; got something else: {:?}", fault),
        }
    }

    #[test]
    fn test_cursor_forward() {
        let index = get_eventual_index(100);
        let mut cursor = index.locate(0);
        let mut count = 0;
        loop {
            // dbg!(&cursor);
            match cursor {
                Location::Indexed(_) => {},
                Location::Gap(GapRange { gap: Unbounded(_), .. }) => break,
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
            count += 1;
            println!("Line {}  Cursor: {} {}", count, index.start_of_line(cursor).unwrap(), index.end_of_line(cursor).unwrap());
            cursor = index.next_line_index(cursor);
        }
        assert_eq!(count, index.lines());
    }

    #[test]
    fn test_cursor_reverse() {
        let index = get_eventual_index(100);
        let mut cursor = index.locate(99);
        let mut count = 0;
        loop {
            match cursor {
                Location::Virtual(VirtualLocation::Start) => break,
                Location::Indexed(_) => {},
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
            // dbg!(&cursor);
            count += 1;
            let (start, end)  = (index.start_of_line(cursor).unwrap(), index.end_of_line(cursor).unwrap());
            println!("Line {}  Cursor: {} {}", count, start, end);
            assert!(start <= end);
            cursor = index.prev_line_index(cursor);
        }
        assert_eq!(count, index.lines());
    }

    #[test]
    fn test_cursor_reverse_gap() {
        let index = get_partial_eventual_index(50, 100);
        let mut cursor = index.locate(149);
        let mut count = 0;
        loop {
            dbg!(&cursor);
            match cursor {
                Location::Indexed(_) => {},
                Location::Gap(GapRange { gap: Bounded(0, 50), .. } ) => break,
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
            count += 1;
            cursor = index.prev_line_index(cursor);
        }
        assert_eq!(count, index.lines());
    }
}

// Cursor functions for EventualIndex
impl EventualIndex {

    // Identify the gap before a given index position and return a Missing() hint to include it.
    // panics if there is no gap
    fn gap_at(&self, pos: usize, target: usize) -> Location {
        self.try_gap_at(pos, target).unwrap()
    }

    // Returns None if there is no gap
    fn try_gap_at(&self, pos: usize, target: usize) -> Option<Location> {
        assert!(pos <= self.indexes.len());
        if self.indexes.is_empty() {
            Some(Location::Gap(GapRange { target, gap: Unbounded(0) } ))
        } else if pos == 0 {
            // gap is at start of file
            let next = self.indexes[pos].start;
            if next > 0 {
                Some(Location::Gap(GapRange { target, gap: Bounded(0, next) } ))
            } else {
                // There is no gap at start of file
                None
            }
        } else {
            let prev = self.indexes[pos-1].end;
            if pos == self.indexes.len() {
                // gap is at end of file; return unbounded range
                Some(Location::Gap(GapRange { target, gap: Unbounded(prev) } ))
            } else {
                // Find the gap between two indexes; bracket result by their [end, start)
                let next = self.indexes[pos].start;
                if next > prev {
                    Some(Location::Gap(GapRange { target, gap: Bounded(prev, next) } ))
                } else {
                    // There is no gap between these indexes
                    assert!(next == prev);
                    None
                }
            }
        }
    }

    // Find index to EOL that contains a given offset or the gap that needs to be loaded to have it
    pub fn locate(&self, offset: usize) -> Location {
        match self.indexes.binary_search_by(|i| i.contains_offset(&offset)) {
            Ok(found) => {
                let i = &self.indexes[found];
                let line = i.find(offset).unwrap();
                if line < i.len() {
                    Location::Indexed(IndexRef(found, line))
                } else {
                    self.next_line_index(Location::Indexed(IndexRef(found, line-1)))
                }
            },
            Err(after) => {
                // No index holds our offset; it needs to be loaded
                self.gap_at(after, offset)
            }
        }
    }

    // Resolve virtual locations to real indexed or gap locations
    pub fn resolve(&self, find: Location) -> Location {
        match find {
            Location::Virtual(loc) => match loc {
                VirtualLocation::Start => {
                    if let Some(gap) = self.try_gap_at(0, 0) {
                        gap
                    } else {
                        Location::Indexed(IndexRef(0, 0))
                    }
                },
                VirtualLocation::End => {
                    unimplemented!("Don't know the end byte!");
                },
            },
            _ => find,
        }
    }

    // Find index to next EOL after given index
    pub fn next_line_index(&self, find: Location) -> Location {
        let find = self.resolve(find);
        match find {
            Location::Indexed(IndexRef(found, line)) => {
                    // next line is in in the same index
                    assert!(found < self.indexes.len());
                    let i = &self.indexes[found];
                    if line + 1 < i.len() {
                        Location::Indexed(IndexRef(found, line + 1))
                    } else if let Some(gap) = self.try_gap_at(found + 1, i.end) {
                        gap
                    } else {
                        Location::Indexed(IndexRef(found+1, 0))
                    }
                },
            _ => find,
        }
    }

    // Find index to prev EOL before given index
    fn prev_line_index(&self, find: Location) -> Location {
        if let Location::Indexed(IndexRef(found, line)) = find {
            // next line is in the same index
            assert!(found < self.indexes.len());
            if line > 0 {
                Location::Indexed(IndexRef(found, line - 1))
            } else if let Some(gap) = self.try_gap_at(found, self.indexes[found].start.max(1) - 1) {
                gap
            } else if found > 0 {
                let j = &self.indexes[found - 1];
                Location::Indexed(IndexRef(found - 1, j.len() - 1))
            } else {
                // There's no gap before this index, and no lines before it either.  We must be at StartOfFile.
                Location::Virtual(VirtualLocation::Start)
            }
        } else {
            find
        }
    }

    // Return offset of start of indexed line, if known
    pub fn start_of_line(&self, find: Location) -> Option<usize> {
        let find = self.resolve(find);
        match find {
                Location::Indexed(_) => {
                    // This line starts one byte after the previous one ends
                    match self.prev_line_index(find) {
                        // FIXME: Store BOL in indexes so we don't have to special case the edges?
                        Location::Virtual(VirtualLocation::Start) => Some(0),  // virtual line before line 1
                        prev => {
                                let prev_eol = self.end_of_line(prev);
                                if let Some(eol) = prev_eol {
                                    Some(eol + 1)
                                } else {
                                    None
                                }
                            },
                        }
                },
            _ => None,
        }
    }

    // Return offset of end of indexed line, if known
    pub fn end_of_line(&self, find: Location) -> Option<usize> {
        let find = self.resolve(find);
        match find {
            Location::Indexed(IndexRef(found, line)) => {
                    assert!(found < self.indexes.len());
                    let i = &self.indexes[found];
                    Some(i.get(line))
                },
            _ => None,
        }
    }
}
