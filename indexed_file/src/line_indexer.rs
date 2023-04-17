// Generic log file source to discover and iterate individual log lines from a LogFile

use std::fmt;
use std::io::SeekFrom;
use crate::files::LogFile;
use crate::index::Index;
use crate::eventual_index::{EventualIndex, Location, VirtualLocation, GapRange, TargetOffset, Missing::{Bounded, Unbounded}};

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
    rev_pos: Location,
}

impl<'a, LOG> LineIndexerIterator<'a, LOG> {
    fn new(file: &'a mut LineIndexer<LOG>) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
            rev_pos: Location::Virtual(VirtualLocation::End),
        }
    }
}

impl<'a, LOG: LogFile> LineIndexerIterator<'a, LOG> {
    fn iterate(&mut self, pos: Location) -> (Location, Option<usize>) {
        let pos = self.file.resolve_location(pos);

        let ret = pos.offset();
        if self.rev_pos == self.pos {
            // End of iterator when fwd and rev meet
            self.rev_pos = Location::Invalid;
            self.pos = Location::Invalid;
            (Location::Invalid, ret)
        } else {
            (pos, ret)
        }
    }

    // Read a string at a given start from our log source
    fn read_line(&mut self, start: usize) -> std::io::Result<String> {
        self.file.source.seek(SeekFrom::Start(start as u64))?;
        let mut line = String::default();
        match self.file.source.read_line(&mut line) {
            Ok(_) => Ok(line),
            Err(e) => Err(e),
        }
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.pos);
        self.pos = self.file.index.next_line_index(pos);
        ret
    }
}

// Iterate over lines in reverse
impl<'a, LOG: LogFile> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.rev_pos);
        self.rev_pos = self.file.index.prev_line_index(pos);
        ret
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
    fn test_iterator_rev() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter().rev();
        let prev = it.next().unwrap();
        let mut prev = prev;

        // Last line is the empty string after the last \n
        assert_eq!(prev, lines * patt_len );

        for i in it.take(lines - 1) {
            let bol = i;
            println!("{bol} {prev}");
            assert_eq!(prev - bol, patt_len);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev_exhaust() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter().rev();
        let prev = it.next().unwrap();
        let mut prev = prev;

        // Last line is the empty string after the last \n
        assert_eq!(prev, lines * patt_len );

        let mut count = 0;
        for i in it {
            let bol = i;
            println!("{bol} {prev}");
            assert_eq!(prev - bol, patt_len);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, lines);
    }

    #[test]
    fn test_iterator_fwd_rev_meet() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 10;//000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter();
        let prev = it.next().unwrap();
        let mut prev = prev;
        let mut count = 1;

        for _ in 0..lines/2 - 1 {
            let i = it.next().unwrap();
            count += 1;
            println!("{count} {i}");
            let bol = i;
            assert_eq!(bol - prev, patt_len);
            prev = bol;
        }

        // Last line is the empty string after the last \n
        assert_eq!(prev, (lines / 2 - 1) * patt_len );

        let bol_part1 = prev;

        let mut it = it.rev();
        prev = it.next().unwrap();      // Fetch last line offset (actually one past the end)
        assert_eq!(prev, lines * patt_len );

        for _ in 0..lines/2 {
            let i = it.next().unwrap();
            count += 1;
            println!("{count} {i}");
            let bol = i;
            assert_eq!(prev - bol, patt_len);
            prev = bol;
        }

        let bol_part2 = prev;
        assert_eq!(bol_part2 - bol_part1, patt_len);

        // all lines exhausted
        assert!(it.next().is_none());
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
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter_lines();
        let (line, prev) = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        assert_eq!(line, patt);
        for i in it.take(lines - 1) {
            let (line, bol) = i;
            assert_eq!(bol - prev, patt_len);
            assert_eq!(line, patt);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter_lines().rev();
        let (line, prev) = it.next().unwrap();
        let mut prev = prev;

        // Last line is the empty string after the last \n
        assert_eq!(prev, lines * patt_len );
        assert!(line.is_empty());

        for i in it.take(lines - 1) {
            let (line, bol) = i;
            println!("{bol} {prev}");
            assert_eq!(prev - bol, patt_len);
            assert_eq!(line, patt);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev_exhaust() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 3; //6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter_lines().rev();
        let (line, prev) = it.next().unwrap();
        let mut prev = prev;

        // Last line is the empty string after the last \n
        assert_eq!(prev, lines * patt_len );
        assert!(line.is_empty());

        let mut count = 0;
        for i in it {
            let (line, bol) = i;
            println!("{bol} {prev}");
            assert_eq!(prev - bol, patt_len);
            assert_eq!(line, patt);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, lines);
    }

    // #[test]
    // fn test_iterator_fwd_rev_meet() {
    //     let patt = "filler\n";
    //     let patt_len = patt.len();
    //     let lines = 6000;
    //     let file = new_mock_file(patt, patt_len * lines, 100);
    //     let mut file = LineIndexer::new(file);
    //     let mut it = file.iter_lines();
    //     let (line, prev) = it.next().unwrap();
    //     let mut prev = prev;

    //     for i in it.take(lines/2) {
    //         let (line, bol) = i;
    //         assert_eq!(bol - prev, patt_len);
    //         assert_eq!(line, patt);
    //         prev = bol;
    //     }

    //     // Last line is the empty string after the last \n
    //     assert_eq!(prev, lines * patt_len );
    //     assert!(line.is_empty());

    //     for i in it.rev().take(lines/2) {
    //         let (line, bol) = i;
    //         assert_eq!(prev - bol, patt_len);
    //         assert_eq!(line, patt);
    //         prev = bol;
    //     }

    //     // all lines exhausted
    //     assert!(it.next().is_none());
    // }

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
            assert_eq!(line, patt);
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
                assert_eq!(line, patt);
                prev = bol;
            }
        }
    }
}

struct LineIndexerDataIterator<'a, LOG> {
    inner: LineIndexerIterator<'a, LOG>,
}

impl<'a, LOG> LineIndexerDataIterator<'a, LOG> {
    fn new(inner: LineIndexerIterator<'a, LOG>) -> Self {
        Self {
            inner,
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

impl<'a, LOG: LogFile>  LineIndexerDataIterator<'a, LOG> {
    fn iterate(&mut self, value: Option<usize>) -> Option<(String, usize)> {
        if let Some(bol) = value {
            // FIXME: Return Some<Result<(offset, String)>> similar to ReadBuf::lines()
            let line = self.inner.read_line(bol).expect("TODO: return Result");
            Some((line, bol))
        } else {
            None
        }
    }
}

// Iterate over lines as position, string
impl<'a, LOG: LogFile> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ret = self.inner.next_back();
        self.iterate(ret)
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = (String, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.inner.next();
        self.iterate(ret)
    }
}

impl<LOG: LogFile> LineIndexer<LOG> {

    pub fn new(file: LOG) -> LineIndexer<LOG> {
        Self {
            source: file,
            index: EventualIndex::new(),
        }
    }

    // Resolve virtual locations to real indexed or gap locations
    #[inline]
    fn resolve(&self, find: Location) -> Location {
        self.index.resolve(find, self.source.len())
    }

    // fill in any gaps by parsing data from the file when needed
    fn resolve_location(&mut self, pos: Location) -> Location {
        // Resolve any virtuals into gaps.
        let mut pos = self.resolve(pos);

        // Resolve gaps
        loop {
            match pos {
                Location::Gap(_) => pos = self.index_chunk(pos),
                _ => break,
            };
        }
        pos
    }


    fn index_chunk(&mut self, gap: Location) -> Location {
        self.source.quench();
        use Location::*;
        use VirtualLocation::*;
        use TargetOffset::*;
        let (target, start, end) = match gap {
            Gap(GapRange { target, gap: Bounded(start, end) }) => (target, start, end.min(self.source.len())),
            Gap(GapRange { target, gap: Unbounded(start) }) => (target, start, self.source.len()),
            Virtual(Start) => (AtOrBefore(0), 0, self.index.start()),
            Virtual(End) => (AtOrBefore(self.source.len().saturating_sub(1)), self.index.end(), self.source.len() ),
            Indexed(_) => panic!("Tried to index a loaded chunk"),
            Invalid => panic!("No invalid locations allowed"),
        };

        let offset = target.value();
        assert!(start <= offset);
        assert!(end <= self.source.len());

        // If Virtual && start == end, don't try to index any chunk, because there is no gap
        match gap {
            Virtual(_) => if start == end { return self.index.locate(target) },
            _ => {},
        };

        if start >= end {
            // FIXME: If this is appropriate, how do we get to Start?
            Location::Invalid
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
            // FIXME: We don't need to do this binary-search lookup if we know where we hit the gap.  Can Gap carry the hint?
            self.index.locate(target)
        }
    }

    fn count_bytes(&self) -> usize {
        self.source.len()
    }

    pub fn count_lines(&self) -> usize {
        self.index.lines()
    }

    fn iter(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        LineIndexerIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.iter()
    }

    pub fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = (String, usize)> + '_ {
        LineIndexerDataIterator::new(LineIndexerIterator::new(self))
    }

}
