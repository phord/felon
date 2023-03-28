// Generic log file source to discover and iterate individual log lines from a LogFile

use std::fmt;
use crate::files::{LogFile, LogFileTrait};
use crate::index::Index;
use crate::eventual_index::{EventualIndex, Location, VirtualLocation, GapRange, Missing::{Bounded, Unbounded}};

pub struct LineIndexer {
    // pub file_path: PathBuf,
    file: LogFile,
    index: EventualIndex,
}

impl fmt::Debug for LineIndexer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineIndexer")
         .field("bytes", &self.count_bytes())
         .field("lines", &self.count_lines())
         .finish()
    }
}

struct LineIndexerIterator<'a> {
    file: &'a mut LineIndexer,
    pos: Location,
}

impl<'a> LineIndexerIterator<'a> {
    fn new(file: &'a mut LineIndexer) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

impl<'a> Iterator for LineIndexerIterator<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.pos = self.file.index.resolve(self.pos);

        loop {
            match self.pos {
                Location::Gap(gap) => self.pos = self.file.index_chunk(gap),
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

// Tests for LineIndexerIterator
#[cfg(test)]
mod logfile_iterator_tests {
    use super::LogFile;
    use super::LineIndexer;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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


// Tests for LineIndexerDataIterator
#[cfg(test)]
mod logfile_data_iterator_tests {
    use super::LogFile;
    use super::LineIndexer;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let trim_patt = &patt[..patt_len-1];
        let lines = 6000;
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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
        let file = LogFile::new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
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

struct LineIndexerDataIterator<'a> {
    file: &'a mut LineIndexer,
    pos: Location,
}

impl<'a> LineIndexerDataIterator<'a> {
    fn new(file: &'a mut LineIndexer) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

// Iterate over lines as position, string
impl<'a> Iterator for LineIndexerDataIterator<'a> {
    type Item = (String, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        self.pos = self.file.index.resolve(self.pos);

        loop {
            match self.pos {
                Location::Gap(gap) => self.pos = self.file.index_chunk(gap),
                Location::Indexed(_) => break,
                Location::Virtual(VirtualLocation::End) => return None,
                Location::Virtual(_) => panic!("Still?"),
            };
        }
        if let Some(bol) = self.file.index.start_of_line(self.pos) {
            if let Some(eol) = self.file.index.end_of_line(self.pos) {
                if let Some(line) = self.file.readline_fixed(bol, eol + 1) {
                    self.pos = self.file.index.next_line_index(self.pos);
                    return Some((line, bol, eol + 1));
                } else {
                    panic!("Unhandled file read error?");
                }
            }
        }
        unreachable!();
    }

}

impl LineIndexer {

    pub fn new(file: LogFile) -> LineIndexer {
        Self {
            file,
            index: EventualIndex::new(),
        }
    }

    fn index_chunk(&mut self, gap: GapRange) -> Location {
        let (offset, start, end) = match gap {
            GapRange { target, gap: Bounded(start, end) } => (target, start, end.min(self.file.len())),
            GapRange { target, gap: Unbounded(start) } => (target, start, self.file.len()),
        };

        assert!(start <= offset);
        assert!(end <= self.file.len());

        if start >= end {
            Location::Virtual(VirtualLocation::End)
        } else {
            let (chunk_start, chunk_end) = self.file.chunk(offset);
            let start = start.max(chunk_start);
            let end = end.min(chunk_end);

            assert!(start <= offset);
            assert!(offset < end);

            // Send the buffer to the parsers
            let buffer = self.file.read(start, end-start).unwrap();
            let mut index = Index::new();
            index.parse(&buffer, start);
            self.index.merge(index);

            self.index.finalize();
            self.index.locate(offset)
        }
    }

    fn count_bytes(&self) -> usize {
        self.file.len()
    }

    pub fn count_lines(&self) -> usize {
        self.index.lines()
    }

    pub fn readline_fixed(&mut self, start: usize, end: usize) -> Option<String> {
        if end <= self.file.len() {
            assert!(end > start);
            // FIXME: Handle unwrap error
            // FIXME: Handle CR+LF endings
            // FIXME: Can't read last byte of file (for the case where it's not EOL)
            Some(String::from_utf8(self.file.read(start, end - start - 1).unwrap()).unwrap())
        } else {
            None
        }
    }

    pub fn iter(&mut self) -> impl Iterator<Item = (usize, usize)> + '_ {
        LineIndexerIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.iter()
    }

    pub fn iter_lines(&mut self) -> impl Iterator<Item = (String, usize, usize)> + '_ {
        LineIndexerDataIterator::new(self)
    }

}
