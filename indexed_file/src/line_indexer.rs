// Structs to index lines in a text file
// TODO: Cleanup - This is a clone of indexer (LogFile) that doesn't parse out words and numbers.  It only parses lines.
//       Needs to be allowed to run in the background better, in a way that Rust can accept.

use std::fmt;
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

type TargetOffset = usize;

#[derive(Debug, Copy, Clone)]
enum GapRange {
    // Position is not indexed; need to index region from given `start` to `end` to resolve offset
    Missing(TargetOffset, OffsetRange),

    // Position is not indexed; unknown gap size at end of index needs to be loaded; arg is first unindexed byte
    MissingUnbounded(TargetOffset, usize),
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
            Location::Gap(GapRange::Missing(_, OffsetRange(0, 50))) => {},
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
            Location::Gap(GapRange::MissingUnbounded(_, _)) => {},
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
                Location::Gap(GapRange::MissingUnbounded(_,_)) => break,
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
                Location::Gap(GapRange::Missing(_, OffsetRange(0,50))) => break,
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
            Some(Location::Gap(GapRange::MissingUnbounded(target, 0)))
        } else if pos == 0 {
            // gap is at start of file
            let next = self.indexes[pos].start;
            if next > 0 {
                Some(Location::Gap(GapRange::Missing(target, OffsetRange(0, next))))
            } else {
                None
            }
        } else {
            let prev = self.indexes[pos-1].end;
            if pos == self.indexes.len() {
                // gap is at end of file; return unbounded range
                Some(Location::Gap(GapRange::MissingUnbounded(target, prev)))
            } else {
                // There's a gap between two indexes; bracket result by their [end, start)
                let next = self.indexes[pos].start;
                if next > prev {
                    Some(Location::Gap(GapRange::Missing(target, OffsetRange(prev, next))))
                } else {
                    None
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
    fn resolve(&self, find: Location) -> Location {
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
    fn next_line_index(&self, find: Location) -> Location {
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
    fn start_of_line(&self, find: Location) -> Option<usize> {
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
    fn end_of_line(&self, find: Location) -> Option<usize> {
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


// NEXT: Replace LogFileLines with something that (generically) loads the EventualIndex on-demand by parsing
// sections as-needed.  If we're smart we can predefine chunks to be demand-loaded that we can later replace
// with zstdlib::frame offsets in some other implementation.
// Our caller can decide when he needs to demand-load everything for searching, counting lines, etc.
// fn load_chunk(offset:usize) -> OffsetRange
//
// Some outer wrapper can hold a cache of recently loaded chunks.

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
    file: &'a mut LogFileLines,
    pos: Location,
}

impl<'a> LogFileLinesIterator<'a> {
    fn new(file: &'a mut LogFileLines) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

impl<'a> Iterator for LogFileLinesIterator<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.pos = self.file.index.resolve(self.pos);

        // FIXME: Get this from self.file; don't even pass it to index_chunk
        let chunk_size = 1024;

        loop {
            match self.pos {
                Location::Gap(gap) => self.pos = self.file.index_chunk(gap, chunk_size),
                Location::Indexed(_) => break,
                Location::Virtual(VirtualLocation::End) => return None,
                Location::Virtual(_) => panic!("Still?"),
            };
        }
        if let Some(bol) = self.file.index.start_of_line(self.pos) {
            if let Some(eol) = self.file.index.end_of_line(self.pos) {
                self.pos = self.file.index.next_line_index(self.pos);
                return Some((bol, eol + 1));
            }
        }
        unreachable!();
    }
}

// Tests for LogFileIterator
#[cfg(test)]
mod logfile_iterator_tests {
    use super::LogFile;
    use super::LogFileLines;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut it = file.iter();
        let (prev, _) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(lines - 1) {
            let (bol, eol) = i;
            assert_eq!(bol - prev, patt_len);
            assert_eq!(eol - bol, patt_len);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_exhaust() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut count = 0;
        for _ in file.iter() {
            count += 1;
        }
        assert_eq!(count, lines);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut count = 0;
        for _ in file.iter() {
            count += 1;
        }
        assert_eq!(count, lines);

        let mut it = file.iter();
        // Iterate again and measure per-line and offsets
        let (prev, _) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(lines - 1) {
            let (bol, eol) = i;
            assert_eq!(bol - prev, patt_len);
            assert_eq!(eol - bol, patt_len);
            prev = bol;
        }
    }


    #[test]
    fn test_iterator_exhaust_half_and_twice() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut count = 0;
        for _ in file.iter().take(lines/2) {
            count += 1;
        }
        assert_eq!(count, lines/2);

        for _ in 0..2 {
            let mut it = file.iter();
            // Iterate again and measure per-line and offsets
            let (prev, _) = it.next().unwrap();
            let mut prev = prev;
            assert_eq!(prev, 0);
            for i in it.take(lines - 1) {
                let (bol, eol) = i;
                assert_eq!(bol - prev, patt_len);
                assert_eq!(eol - bol, patt_len);
                prev = bol;
            }
        }
    }
}


// Tests for LogFileIterator
#[cfg(test)]
mod logfile_data_iterator_tests {
    use super::LogFile;
    use super::LogFileLines;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let trim_patt = &patt[..patt_len-1];
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut it = file.iter_lines();
        let (line, prev, _) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        assert_eq!(line, trim_patt);
        for i in it.take(lines - 1) {
            let (line, bol, eol) = i;
            assert_eq!(bol - prev, patt_len);
            assert_eq!(eol - bol, patt_len);
            assert_eq!(line, trim_patt);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_exhaust() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut count = 0;
        for _ in file.iter_lines() {
            count += 1;
        }
        assert_eq!(count, lines);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let trim_patt = &patt[..patt_len-1];
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut count = 0;
        for _ in file.iter_lines() {
            count += 1;
        }
        assert_eq!(count, lines);

        let mut it = file.iter_lines();
        // Iterate again and measure per-line and offsets
        let (_, prev, _) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(lines - 1) {
            let (line, bol, eol) = i;
            assert_eq!(bol - prev, patt_len);
            assert_eq!(eol - bol, patt_len);
            assert_eq!(line, trim_patt);
            prev = bol;
        }
    }


    #[test]
    fn test_iterator_exhaust_half_and_twice() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let trim_patt = &patt[..patt_len-1];
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines);
        let mut file = LogFileLines::new(file);
        let mut count = 0;
        for _ in file.iter_lines().take(lines/2) {
            count += 1;
        }
        assert_eq!(count, lines/2);

        for _ in 0..2 {
            let mut it = file.iter_lines();
            // Iterate again and measure per-line and offsets
            let (_, prev, _) = it.next().unwrap();
            let mut prev = prev;
            assert_eq!(prev, 0);
            for i in it.take(lines - 1) {
                let (line, bol, eol) = i;
                assert_eq!(bol - prev, patt_len);
                assert_eq!(eol - bol, patt_len);
                assert_eq!(line, trim_patt);
                prev = bol;
            }
        }
    }
}

struct LogFileDataIterator<'a> {
    file: &'a mut LogFileLines,
    pos: Location,
}

impl<'a> LogFileDataIterator<'a> {
    fn new(file: &'a mut LogFileLines) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

// Iterate over lines as position, string
impl<'a> Iterator for LogFileDataIterator<'a> {
    type Item = (String, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.pos = self.file.index.resolve(self.pos);

        // FIXME: Get this from self.file; don't even pass it to index_chunk
        let chunk_size = 1024;

        loop {
            match self.pos {
                Location::Gap(gap) => self.pos = self.file.index_chunk(gap, chunk_size),
                Location::Indexed(_) => break,
                Location::Virtual(VirtualLocation::End) => return None,
                Location::Virtual(_) => panic!("Still?"),
            };
        }
        if let Some(bol) = self.file.index.start_of_line(self.pos) {
            if let Some(eol) = self.file.index.end_of_line(self.pos) {
                if let Some(line) = self.file.readline_fixed(bol, eol + 1) {
                    self.pos = self.file.index.next_line_index(self.pos);
                    return Some((line.to_string(), bol, eol + 1));
                } else {
                    panic!("Unhandled file read error?");
                }
            }
        }
        unreachable!();
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

    fn index_chunk(&mut self, gap: GapRange, chunk_size: usize) -> Location {
        let (offset, start, end) = match gap {
            GapRange::Missing(offset, OffsetRange(start, end)) => (offset, start, end),
            GapRange::MissingUnbounded(offset, start) => (offset, start, start + self.chunk_size),
        };

        assert!(start <= offset);
        assert!(offset < end);
        assert!(start < end);

        let end = end.min(self.file.len());

        if start < end {
            let (start, end) =
                if end - start <= chunk_size {
                    (start, end)
                } else {
                    let offset = offset.max(start).min(end);
                    if offset - start <= chunk_size {
                        // Prefer to load near the front
                        (start, start + chunk_size)
                    } else if end - offset <= chunk_size {
                        // But load near the end if it's closer to our target
                        (end - chunk_size, end)
                    } else {
                        // otherwise, load the chunk centered around our target
                        let start = offset - chunk_size / 2;
                        (start, start + chunk_size)
                    }
                };

            // Send the buffer to the parsers
            let buffer = self.file.read(start, end-start).unwrap();
            let mut index = Index::new();
            index.parse(buffer, start);
            self.index.merge(index);

            self.index.finalize();
            self.index.locate(offset)
        } else {
            Location::Virtual(VirtualLocation::End)
        }
    }

    fn count_bytes(&self) -> usize {
        self.file.len()
    }

    pub fn count_lines(&self) -> usize {
        self.index.lines()
    }

    pub fn readline_fixed<'a>(&'a self, start: usize, end: usize) -> Option<&'a str> {
        if end <= self.file.len() {
            assert!(end > start);
            // FIXME: Handle unwrap error
            // FIXME: Handle CR+LF endings
            // FIXME: Can't read last byte of file (for the case where it's not EOL)
            Some(std::str::from_utf8(self.file.read(start, end - start - 1).unwrap()).unwrap())
        } else {
            None
        }
    }

    pub fn iter(&mut self) -> impl Iterator<Item = (usize, usize)> + '_ {
        LogFileLinesIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.iter()
    }

    pub fn iter_lines(&mut self) -> impl Iterator<Item = (String, usize, usize)> + '_ {
        LogFileDataIterator::new(self)
    }

}
