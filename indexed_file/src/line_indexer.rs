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
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        // FIXME: Move this into resolve_location?
        self.pos = self.file.index.resolve(self.pos);
        self.pos = self.file.resolve_location(self.pos);

        if let Some(bol) = self.file.index.start_of_line(self.pos) {
            self.pos = self.file.index.next_line_index(self.pos);
            Some(bol)
        } else {
            None
        }
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
        let prev = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(lines - 1) {
            let bol = i;
            assert_eq!(bol - prev, patt_len);
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
        assert_eq!(count, lines + 1);
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
        assert_eq!(count, lines + 1);

        let mut it = file.iter();
        // Iterate again and measure per-line and offsets
        let prev = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(lines - 1) {
            let bol = i;
            assert_eq!(bol - prev, patt_len);
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
            let prev = it.next().unwrap();
            let mut prev = prev;
            assert_eq!(prev, 0);
            for i in it.take(lines - 1) {
                let bol = i;
                assert_eq!(bol - prev, patt_len);
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
        let (line, prev) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        assert_eq!(line, trim_patt);
        for i in it.take(lines - 1) {
            let (line, bol) = i;
            assert_eq!(bol - prev, patt_len);
            assert_eq!(line, trim_patt);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let trim_patt = &patt[..patt_len-1];
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter_lines().rev();
        let (line, prev) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, (lines-1) * patt_len );
        assert_eq!(line, trim_patt);
        for i in it.take(lines - 1) {
            let (line, bol) = i;
            assert_eq!(prev - bol, patt_len);
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
        assert_eq!(count, lines + 1);
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
        assert_eq!(count, lines + 1);

        let mut it = file.iter_lines();
        // Iterate again and measure per-line and offsets
        let (_, prev) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(lines - 1) {
            let (line, bol) = i;
            assert_eq!(bol - prev, patt_len);
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
            let (_, prev) = it.next().unwrap();
            let mut prev = prev;
            assert_eq!(prev, 0);
            for i in it.take(lines - 1) {
                let (line, bol) = i;
                assert_eq!(bol - prev, patt_len);
                assert_eq!(line, trim_patt);
                prev = bol;
            }
        }
    }
}

struct LineIndexerDataIterator<'a, LOG> {
    file: &'a mut LineIndexer<LOG>,
    pos: Location,
    rev_pos: Location,
}

impl<'a, LOG> LineIndexerDataIterator<'a, LOG> {
    fn new(file: &'a mut LineIndexer<LOG>) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
            rev_pos: Location::Virtual(VirtualLocation::End),
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


// Read a string at a given start from our log source
fn read_line<LOG: LogFile>(file: &mut LOG, start: usize) -> std::io::Result<String> {
    file.seek(SeekFrom::Start(start as u64))?;
    let mut line = String::default();
    match file.read_line(&mut line) {
        Ok(0) => Ok(line),
        Ok(_n) => {
            if line.ends_with('\n') {
                line.pop();
                if line.ends_with('\r') {
                    line.pop();
                }
            }
            Ok(line)
        }
        Err(e) => Err(e),
    }
}

impl<'a, LOG: LogFile> LineIndexerDataIterator<'a, LOG> {
    // Necessary dup?

    fn resolve(&mut self, pos: Location) -> (Location, Option<(String, usize)>) {
        // FIXME: Move this into resolve_location?
        let pos = self.file.index.resolve(pos);
        let pos = self.file.resolve_location(pos);

        if let Some(bol) = self.file.index.start_of_line(pos) {
            let line = read_line(&mut self.file.source, bol).expect("Unhandled file read error");
            (pos, Some((line, bol)))
        } else {
            (pos, None)
        }
    }
}

// Iterate over lines as position, string
impl<'a, LOG: LogFile> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.resolve(self.rev_pos);
        self.rev_pos = pos;
        match ret {
            Some(_) => self.rev_pos = self.file.index.prev_line_index(self.rev_pos),
            _ => {},
        }
        ret
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = (String, usize);

    // FIXME: Return Some<Result<(offset, String)>> similar to ReadBuf::lines()
    fn next(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.resolve(self.pos);
        self.pos = pos;
        match ret {
            Some(_) => self.pos = self.file.index.next_line_index(self.pos),
            _ => {},
        }
        ret
    }

}

impl<LOG: LogFile> LineIndexer<LOG> {

    pub fn new(file: LOG) -> LineIndexer<LOG> {
        Self {
            source: file,
            index: EventualIndex::new(),
        }
    }

    // fill in any gaps by parsing data from the file when needed
    fn resolve_location(&mut self, pos: Location) -> Location {
        let mut pos = pos;
        loop {
            match pos {
                Location::Indexed(_) => return pos,
                Location::Virtual(_) => return pos,
                p => pos = self.index_chunk(p),
            };
        }

    }
    fn index_chunk(&mut self, gap: Location) -> Location {
        self.source.quench();

        let (offset, start, end) = match gap {
            Location::Gap(GapRange { target, gap: Bounded(start, end) }) => (target, start, end.min(self.source.len())),
            Location::Gap(GapRange { target, gap: Unbounded(start) }) => (target, start, self.source.len()),
            Location::Virtual(VirtualLocation::Start) => (0, 0, self.index.start()),
            Location::Virtual(VirtualLocation::End) => (self.source.len().saturating_sub(1), self.index.end(), self.source.len() ),
            Location::Indexed(_) => panic!("Tried to index a loaded chunk"),
        };

        assert!(start <= offset);
        assert!(end <= self.source.len());

        // If Location::Virtual && start == end, don't try to index any chunk
        match gap {
            Location::Virtual(_) => if start == end { return self.index.locate(offset) },
            _ => {},
        };

        if start >= end {
            // FIXME: If this is appropriate, how do we get to Start?
            Location::Virtual(VirtualLocation::End)
        } else {
            let (chunk_start, chunk_end) = self.source.chunk(offset);
            let start = start.max(chunk_start);
            let end = end.min(chunk_end);

            assert!(start <= offset);
            assert!(offset <= end);

            // Send the buffer to the parsers
            self.source.seek(SeekFrom::Start(start as u64)).expect("Seek does not fail");
            let mut index = Index::new();
            index.parse_bufread(&mut self.source, start, end - start).expect("Ignore read errors");
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

    pub fn iter(&mut self) -> impl Iterator<Item = usize> + '_ {
        LineIndexerIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl Iterator<Item = usize> + '_ {
        self.iter()
    }

    pub fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = (String, usize)> + '_ {
        LineIndexerDataIterator::new(self)
    }

}
