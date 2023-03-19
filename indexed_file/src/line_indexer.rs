// Structs to index lines in a text file
// TODO: Cleanup - This is a clone of indexer (LogFile) that doesn't parse out words and numbers.  It only parses lines.
//       Needs to be allowed to run in the background better, in a way that Rust can accept.

use std::fmt;
use crate::log_file::{LogFile, LogFileTrait};
use crate::index::Index;
use crate::eventual_index::{EventualIndex, Location, VirtualLocation, GapRange, Missing::{Bounded, Unbounded}};

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
            GapRange { target, gap: Bounded(start, end) } => (target, start, end),
            GapRange { target, gap: Unbounded(start) } => (target, start, start + self.chunk_size),
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
