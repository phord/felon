// Structs to index lines in a text file
// TODO: Cleanup - This is a clone of indexer (LogFile) that doesn't parse out words and numbers.  It only parses lines.
//       Needs to be allowed to run in the background better, in a way that Rust can accept.

use std::fmt;
use crossbeam::scope;
use crossbeam_channel::{bounded, unbounded};
use crate::log_file::{LogFile, LogFileTrait};
use crate::index::Index;


struct EventualIndex {
    indexes: Vec<Index>,
}

impl EventualIndex {
    fn new() -> EventualIndex {
        EventualIndex {
            indexes: Vec::new(),
        }
    }

    fn merge(&mut self, other: Index) {
        // merge lazily
        self.indexes.push(other);
    }

    fn finalize(&mut self) {
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
    }

    fn line_offset(&self, line_number: usize) -> Option<usize> {
        if line_number >= self.lines() {
            return None;
        }
        self.iter().skip(line_number).cloned().next()
    }

    pub fn iter(self: &Self) -> impl DoubleEndedIterator<Item = &usize> {
        self.indexes.iter().flat_map(|x| x.iter())
    }

    fn bytes(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.bytes())
    }

    fn lines(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.lines())
    }
}

// Delineates [start, end) of a region of the file.  end is not inclusive.
#[derive(Debug, Copy, Clone)]
struct OffsetRange(usize, usize);

// Literally a reference by subscript to the Index/Line in an EventualIndex.
// Becomes invalid if the EventualIndex changes, but since we use this as a hint only, it's not fatal.
#[derive(Debug, Copy, Clone)]
struct IndexRef(usize, usize);

#[derive(Debug, Copy, Clone)]
enum VirtualLocation {
    Start,
    End
}

#[derive(Debug, Copy, Clone)]
enum GapRange {
    // Position is not indexed; need to index region from given `start` to `end`
    Missing(OffsetRange),

    // Position is not indexed; unknown gap size at end of index needs to be loaded; arg is first unindexed byte
    MissingUnbounded(usize),
}

#[derive(Debug, Copy, Clone)]
enum Location {
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
    use super::VirtualLocation;
    use super::IndexRef;
    use super::OffsetRange;
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
            Location::Virtual(VirtualLocation::Start) => {},
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
            _ => panic!("Expected Index(0,0); got something else: {:?}", cursor),
        }
        let fault = index.locate(10);
        match fault {
            Location::Gap(GapRange::Missing(OffsetRange(0, 50))) => {},
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
            Location::Gap(GapRange::MissingUnbounded(_)) => {},
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
                Location::Virtual(VirtualLocation::Start) => {},
                Location::Gap(GapRange::MissingUnbounded(_)) => break,
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
            // dbg!(&cursor);
            count += 1;
            println!("Line {}  Cursor: {} {}", count, index.start_of_line(cursor).unwrap(), index.end_of_line(cursor).unwrap());
            match cursor {
                Location::Indexed(_) => {},
                Location::Virtual(VirtualLocation::Start) => break,
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
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
                Location::Gap(GapRange::Missing(OffsetRange(0,50))) => break,
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
    fn gap_at(&self, pos: usize) -> Location {
        assert!(pos <= self.indexes.len());
        if self.indexes.is_empty() {
            Location::Gap(GapRange::MissingUnbounded(0))
        } else if pos == 0 {
            // gap is at start of file
            let next = self.indexes[pos].start;
            if next > 0 {
                Location::Gap(GapRange::Missing(OffsetRange(0, next)))
            } else {
                unreachable!()
            }
        } else {
            let prev = self.indexes[pos-1].end;
            if pos == self.indexes.len() {
                // gap is at end of file; return unbounded range
                Location::Gap(GapRange::MissingUnbounded(prev))
            } else {
                // There's a gap between two indexes; bracket result by their [end, start)
                let next = self.indexes[pos].start;
                if next > prev {
                    Location::Gap(GapRange::Missing(OffsetRange(prev, next)))
                } else {
                    unreachable!()
                }
            }
        }
    }

    // Find index to EOL that contains a given offset or the gap that needs to be loaded to have it
    fn locate(&self, offset: usize) -> Location {
        match self.indexes.binary_search_by(|i| i.contains_offset(&offset)) {
            Ok(found) => {
                let i = &self.indexes[found];
                let line = i.find(offset).unwrap();
                if line == 0 && i.start == 0 {
                    Location::Virtual(VirtualLocation::Start)
                } else if line < i.len() {
                    Location::Indexed(IndexRef(found, line))
                } else {
                    self.next_line_index(Location::Indexed(IndexRef(found, line-1)))
                }
            },
            Err(after) => {
                // No index holds our offset; it needs to be loaded
                self.gap_at(after)
            }
        }
    }

    // True if there is no gap between given index and the next one
    fn next_is_contiguous(&self, pos: usize) -> bool {
        assert!(pos < self.indexes.len());
        pos + 1 < self.indexes.len() && {
            let i = &self.indexes[pos];
            let j = &self.indexes[pos + 1];
            assert!(j.start >= i.end);
            j.start == i.end
        }
    }

    // Find index to next EOL after given index
    fn next_line_index(&self, find: Location) -> Location {
        match find {
            Location::Virtual(VirtualLocation::Start) => {
                    // TODO: Get rid of this weirdo
                    if self.indexes.is_empty() {
                        self.gap_at(0)
                    } else {
                        Location::Indexed(IndexRef(0, 1))
                    }
                },
            Location::Indexed(IndexRef(found, line)) => {
                    // next line is in in the same index
                    assert!(found < self.indexes.len());
                    let i = &self.indexes[found];
                    if line + 1 < i.len() {
                        Location::Indexed(IndexRef(found, line + 1))
                    } else if self.next_is_contiguous(found) {
                        Location::Indexed(IndexRef(found+1, 0))
                    } else {
                        self.gap_at(found + 1)
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
            let i = &self.indexes[found];
            if i.start == 0 && line == 1 {
                Location::Virtual(VirtualLocation::Start)   // TODO: Weird special case for first line in file.
            } else if line > 0 {
                Location::Indexed(IndexRef(found, line - 1))
            } else if found > 0 && self.next_is_contiguous(found - 1) {
                let j = &self.indexes[found - 1];
                Location::Indexed(IndexRef(found - 1, j.len() - 1))
            } else {
                self.gap_at(found)
            }
        } else {
            find
        }
    }

    // Return offset of start of indexed line, if known
    fn start_of_line(&self, find: Location) -> Option<usize> {
        match find {
            Location::Virtual(VirtualLocation::Start) => Some(0),

            Location::Indexed(_) => {
                    // This line starts one byte after the previous one ends
                    let find = self.prev_line_index(find);
                    let prev_eol = self.end_of_line(find);
                    if let Some(eol) = prev_eol {
                        Some(eol + 1)
                    } else {
                        None
                    }
                },
            _ => None,
        }
    }

    // Return offset of end of indexed line, if known
    fn end_of_line(&self, find: Location) -> Option<usize> {
        match find {
            Location::Virtual(VirtualLocation::Start) => {
                if self.indexes.is_empty() {
                    None
                } else {
                    let i = &self.indexes[0];
                    Some(i.get(0))
                }
            },

            Location::Indexed(IndexRef(found, line)) => {
                    assert!(found < self.indexes.len());
                    let i = &self.indexes[found];
                    Some(i.get(line))
                },
            _ => None,
        }
    }
}

pub struct LogFileLines {
    // pub file_path: PathBuf,
    file: LogFile,
    index: EventualIndex,
    chunk_size: usize,
}

impl fmt::Debug for LogFileLines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogFileLines")
         .field("bytes", &self.count_bytes())
         .field("lines", &self.count_lines())
         .field("chunk_size", &self.chunk_size)
         .finish()
    }
}

struct LogFileLinesIterator<'a> {
    file: &'a LogFileLines,
    pos: Location,
}

impl<'a> LogFileLinesIterator<'a> {
    fn new(file: &'a LogFileLines) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

impl<'a> Iterator for LogFileLinesIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // FIXME: Let StartOfFile be a hole that leads to IndexOffset(0,0)?
        self.pos = self.file.index.next_line_index(self.pos);

        loop {
            match self.pos {
                Location::Gap(_) => todo!(),
                Location::Indexed(_) => break,
                Location::Virtual(_) => panic!("Still?"),
            };
        }
        self.file.index.start_of_line(self.pos)
    }
}

#[test]
fn test_iterator() {
    let file = LogFile::new_mock_file("filler\n", 10000);
    let mut file = LogFileLines::new(file);
    for i in file.iter().take(5) {
        println!("{}", i);
    }
}

impl LogFileLines {

    pub fn new(file: LogFile) -> LogFileLines {
        Self {
            file,
            index: EventualIndex::new(),
            chunk_size: 1024 * 1024 * 1,
        }
    }

    fn index_chunk(&mut self, gap: GapRange, near_offset: usize) {
        // TODO: find chunk near `near_offset`
        let (start, end) = match gap {
            GapRange::Missing(OffsetRange(start, end)) => (start, end),
            GapRange::MissingUnbounded(start) => (start, start + self.chunk_size),
            _ => (0, 0),
        };

        if start < end {
            // Send the buffer to the parsers
            let buffer = self.file.read(start, end-start).unwrap();
            let mut index = Index::new();
            index.parse(buffer, start);
            self.index.merge(index);
        }

        self.index.finalize();
    }

    fn count_bytes(&self) -> usize {
        self.file.len()
    }

    pub fn count_lines(&self) -> usize {
        self.index.lines()
    }

    pub fn readline_fixed(&self, start: usize, end: usize) -> Option<&str> {
        if end < self.file.len() {
            assert!(end > start);
            // FIXME: Handle unwrap error
            // FIXME: Handle CR+LF endings
            Some(std::str::from_utf8(self.file.read(start, end - start - 1).unwrap()).unwrap())
        } else {
            None
        }
    }

    pub fn iter(&mut self) -> impl Iterator<Item = usize> + '_ {
        LogFileLinesIterator::new(self)
    }

    pub fn iter_offsets(&self) -> impl Iterator<Item = (&usize, &usize)> + '_ {
        let starts = std::iter::once(&0usize).chain(self.index.iter());
        let ends = self.index.iter();
        let line_range = starts.zip(ends);
        line_range
    }

    pub fn iter_lines(&mut self) -> impl Iterator<Item = (&str, usize, usize)> + '_ {
        self.iter_offsets().map(|(&start, &end)| -> (&str, usize, usize) {(self.readline_fixed(start, end).unwrap_or(""), start, end)})
    }

}
