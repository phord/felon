// Generic log file source to discover and iterate individual log lines from a LogFile

use std::fmt;
use std::io::SeekFrom;
use crate::files::LogFile;
use crate::index::Index;
use crate::eventual_index::{EventualIndex, Location, VirtualLocation, GapRange, Missing::{Bounded, Unbounded}};

pub struct LineIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: EventualIndex,
}

impl<LOG: LogFile> fmt::Debug for LineIndexer<LOG> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineIndexer")
         .field("bytes", &self.count_bytes())
         .field("lines", &self.count_lines())
         .finish()
    }
}

struct LineIndexerIterator<'a, LOG> {
    file: &'a mut LineIndexer<LOG>,
    pos: Location,
}

impl<'a, LOG> LineIndexerIterator<'a, LOG> {
    fn new(file: &'a mut LineIndexer<LOG>) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerIterator<'a, LOG> {
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
    use crate::files::new_mock_file;
    use super::LineIndexer;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
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
        let file = new_mock_file(patt, patt_len * lines, 100);
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
        let file = new_mock_file(patt, patt_len * lines, 100);
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
        let file = new_mock_file(patt, patt_len * lines, 100);
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
    use crate::files::new_mock_file;
    use super::LineIndexer;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let trim_patt = &patt[..patt_len-1];
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
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
        let file = new_mock_file(patt, patt_len * lines, 100);
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
        let file = new_mock_file(patt, patt_len * lines, 100);
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
        let file = new_mock_file(patt, patt_len * lines, 100);
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

struct LineIndexerDataIterator<'a, LOG> {
    file: &'a mut LineIndexer<LOG>,
    pos: Location,
}

impl<'a, LOG> LineIndexerDataIterator<'a, LOG> {
    fn new(file: &'a mut LineIndexer<LOG>) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
        }
    }
}

/**
 * TODO: Implement Double-ended iterators that produce Strings for each line of input.
 *
 * TODO: an iterator that iterates lines and builds up the EventualIndex as it goes.
 * TODO: an iterator that iterates from a given line offset forward or reverse.
 *
 * TODO: Can we make a filtered iterator that tests the line in the file buffer and only copy to String if it matches?
 */

// Iterate over lines as position, string
impl<'a, LOG: LogFile> LineIndexerDataIterator<'a, LOG> {

    // Read a string at a given start and len from our log source
    fn read_line(&mut self, start: usize, len: usize) -> Result<String, std::io::Error> {
        self.file.source.seek(SeekFrom::Start(start as u64))?;
        let mut line = String::default();
        let mut length = len as usize;
        line.reserve(length);
        while length != 0 {
            let buf = self.file.source.fill_buf()?;
            let bytes = length.min(buf.len());
            line += &String::from_utf8(buf[..bytes].to_vec()).expect("Don't have utf8 errors"); //.map_err(|e| { Err::new(std::io::ErrorKind::Other) })?;
            self.file.source.consume(bytes);
            length -= bytes;
        }
        Ok(line)
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = (String, usize, usize);

    // FIXME: Return Some<Result<(offset, String)>> similar to ReadBuf::lines()
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

        // FIXME: Let Location::Indexed contain the bol value; then get rid of start_of_line/end_of_line calls here
        if let Some(bol) = self.file.index.start_of_line(self.pos) {
            if let Some(eol) = self.file.index.end_of_line(self.pos) {
                self.file.source.seek(SeekFrom::Start(bol as u64)).expect("Seek does not fail");
                let line = self.read_line(bol, eol - bol).expect("Unhandled file read error");
                self.pos = self.file.index.next_line_index(self.pos);
                return Some((line, bol, eol + 1));
            }
        }
        unreachable!();
    }

}

impl<LOG: LogFile> LineIndexer<LOG> {

    pub fn new(file: LOG) -> LineIndexer<LOG> {
        Self {
            source: file,
            index: EventualIndex::new(),
        }
    }

    fn index_chunk(&mut self, gap: GapRange) -> Location {
        self.source.quench();
        let (offset, start, end) = match gap {
            GapRange { target, gap: Bounded(start, end) } => (target, start, end.min(self.source.len())),
            GapRange { target, gap: Unbounded(start) } => (target, start, self.source.len()),
        };

        assert!(start <= offset);
        assert!(end <= self.source.len());

        if start >= end {
            Location::Virtual(VirtualLocation::End)
        } else {
            let (chunk_start, chunk_end) = self.source.chunk(offset);
            let start = start.max(chunk_start);
            let end = end.min(chunk_end);

            assert!(start <= offset);
            assert!(offset < end);

            // Send the buffer to the parsers
            self.source.seek(SeekFrom::Start(start as u64)).expect("Seek does not fail");
            let mut index = Index::new();
            index.parse_bufread(&mut self.source, start, end).expect("Ignore read errors");
            self.index.merge(index);

            self.index.finalize();
            self.index.locate(offset)
        }
    }

    fn count_bytes(&self) -> usize {
        self.source.len()
    }

    pub fn count_lines(&self) -> usize {
        self.index.lines()
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
