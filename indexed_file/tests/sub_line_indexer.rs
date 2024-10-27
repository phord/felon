// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

#[cfg(test)]
mod sub_line_iterator_helper {
    use indexed_file::{LineViewMode, SubLineIterator, IndexedLog, Log};
    use indexed_file::files::new_mock_file;

    pub(crate) struct Harness {
        pub(crate) patt: String,
        pub(crate) patt_len: usize,
        pub(crate) lines: usize,
    }

    impl Harness {
        pub(crate) fn new(patt: &str, lines: usize, ) -> (Self, Log) {
            let patt_len = patt.len();
            let file = new_mock_file(patt, patt_len * lines, 100);
            let file = Log::from(file);
            let s = Self {
                patt: patt.to_string(),
                patt_len,
                lines,
            };
            (s, file)

        }

        pub(crate) fn total_len(&self, width: usize) -> usize {
            self.lines * ((self.patt_len + width - 1) / width)
        }

        pub(crate) fn offset_into_line(&self, offset: usize) -> usize {
            offset % self.patt_len
        }

        pub(crate) fn expected_bol(&self, offset: usize, width: usize) -> usize {
            let line_ofs = self.offset_into_line(offset);
            offset - line_ofs + line_ofs / width * width
        }

        pub(crate) fn expected_width(&self, offset: usize, width: usize) -> usize {
            let offset = self.expected_bol(offset, width);
            (self.patt_len - self.offset_into_line(offset)).min(width)
        }

        pub(crate) fn expected_line(&self, offset: usize, width: usize) -> &str {
            let offset = self.expected_bol(offset, width);
            let ofs = self.offset_into_line(offset);
            let width = self.expected_width(offset, width);
            &self.patt[ofs..ofs + width]
        }

        pub(crate) fn default() -> (Self, Log) {
            Self::new("abcdefghijklmnopqrstuvwxyz\n", 6000)
        }

        pub(crate) fn new_small(lines: usize) -> (Self, Log) {
            Self::new("abcdefghijklmnopqrstuvwxyz\n", lines)
        }
    }

    pub(crate) fn new<LOG: IndexedLog>(log: &mut LOG) -> SubLineIterator<LOG> {
        let mode = LineViewMode::WholeLine;
        SubLineIterator::new(log, mode)
    }

    pub(crate) fn new_from<LOG: IndexedLog>(log: &mut LOG, offset: usize) -> SubLineIterator<LOG> {
        let mode = LineViewMode::WholeLine;
        SubLineIterator::new_from(log, mode, offset)
    }
}

// Tests for LineIndexerIterator
#[cfg(test)]
mod logfile_iterator_tests {
    use indexed_file::IndexedLog;

    use crate::sub_line_iterator_helper::Harness;

    #[test]
    fn test_iterator() {
        let (harness, mut file) = Harness::default();
        let mut it = file.iter_offsets();
        let prev = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(harness.lines - 1) {
            let bol = i;
            assert_eq!(bol - prev, harness.patt_len);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev() {
        let (harness, mut file) = Harness::default();
        let mut it = file.iter_offsets().rev();
        let prev = it.next().unwrap();
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        for i in it.take(harness.lines - 1) {
            let bol = i;
            println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev_exhaust() {
        let (harness, mut file) = Harness::default();
        let mut it = file.iter_offsets().rev();
        let prev = it.next().unwrap();
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        let mut count = 1;
        for i in it {
            let bol = i;
            // println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_fwd_rev_meet() {
        let (harness, mut file) =  Harness::new_small(10);
        let mut it = file.iter_offsets();
        let prev = it.next().unwrap();
        let mut prev = prev;
        let mut count = 1;

        for _ in 0..harness.lines/2 - 1 {
            let i = it.next().unwrap();
            count += 1;
            println!("{count} {i}");
            let bol = i;
            assert_eq!(bol - prev, harness.patt_len);
            prev = bol;
        }

        // Last line is the empty string after the last \n
        assert_eq!(prev, (harness.lines / 2 - 1) * harness.patt_len );

        let bol_part1 = prev;

        let mut it = it.rev();
        prev = it.next().unwrap();      // Fetch last line offset
        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        for _ in 0..harness.lines/2 - 1 {
            let i = it.next().unwrap();
            count += 1;
            println!("{count} {i}");
            let bol = i;
            assert_eq!(prev - bol, harness.patt_len);
            prev = bol;
        }

        let bol_part2 = prev;
        assert_eq!(bol_part2 - bol_part1, harness.patt_len);

        // all lines exhausted
        assert!(it.next().is_none());
    }

    #[test]
    fn test_iterator_exhaust() {
        let (harness, mut file) = Harness::default();
        let count = file.iter_offsets().count();
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        let (harness, mut file) = Harness::default();
        let count = file.iter_offsets().count();
        assert_eq!(count, harness.lines);

        let mut it = file.iter_offsets();
        // Iterate again and measure per-line and offsets
        let prev = it.next().unwrap();
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(harness.lines - 1) {
            let bol = i;
            assert_eq!(bol - prev, harness.patt_len);
            prev = bol;
        }
    }


    #[test]
    fn test_iterator_exhaust_half_and_twice() {
        let (harness, mut file) = Harness::default();
        let count = file.iter_offsets().take(harness.lines/2).count();
        assert_eq!(count, harness.lines/2);

        for _ in 0..2 {
            let mut it = file.iter_offsets();
            // Iterate again and measure per-line and offsets
            let prev = it.next().unwrap();
            let mut prev = prev;
            assert_eq!(prev, 0);
            for i in it.take(harness.lines - 1) {
                let bol = i;
                assert_eq!(bol - prev, harness.patt_len);
                prev = bol;
            }
        }
    }
}


// Tests for SubLineIterator
#[cfg(test)]
mod sub_line_iterator_tests {
    use std::collections::HashSet;
    use crate::sub_line_iterator_helper::{self, Harness};


    #[test]
    fn test_iterator() {
        let (harness, mut file) = Harness::default();
        let mut it = sub_line_iterator_helper::new(&mut file);
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);
        let mut prev = prev;
        assert_eq!(prev, 0);
        assert_eq!(line, harness.patt);
        for i in it.take(harness.lines - 1) {
            let (line, bol) = (i.line, i.offset);
            assert_eq!(bol - prev, harness.patt_len);
            assert_eq!(line, harness.patt);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev() {
        let (harness, mut file) = Harness::default();
        let mut it = sub_line_iterator_helper::new(&mut file).rev();
        let line = it.next().unwrap();
        let (_line, prev) = (line.line, line.offset);
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        for i in it.take(harness.lines - 2) {
            let (line, bol) = (i.line, i.offset);
            // println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            assert_eq!(line, harness.patt);
            prev = bol;
        }
    }

    #[test]
    fn test_iterator_rev_exhaust() {
        let (harness, mut file) =  Harness::new_small(3);
        let mut it = sub_line_iterator_helper::new(&mut file).rev();
        let line = it.next().unwrap();
        let (_line, prev) = (line.line, line.offset);

        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        let mut count = 1;
        for i in it {
            let (line, bol) = (i.line, i.offset);
            println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            assert_eq!(line, harness.patt);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_exhaust() {
        let (harness, mut file) = Harness::default();
        let count = sub_line_iterator_helper::new(&mut file).count();
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        let (harness, mut file) = Harness::default();
        let count = sub_line_iterator_helper::new(&mut file).count();
        assert_eq!(count, harness.lines);

        let mut it = sub_line_iterator_helper::new(&mut file);
        // Iterate again and measure per-line and offsets
        let line = it.next().unwrap();
        let mut prev = line.offset;
        assert_eq!(prev, 0);
        for i in it.take(harness.lines - 1) {
            let (line, bol) = (i.line, i.offset);
            assert_eq!(bol - prev, harness.patt_len);
            assert_eq!(line, harness.patt);
            prev = bol;
        }
    }


    #[test]
    fn test_iterator_exhaust_half_and_twice() {
        let (harness, mut file) = Harness::default();
        let count = sub_line_iterator_helper::new(&mut file).take(harness.lines/2).count();
        assert_eq!(count, harness.lines/2);

        for _ in 0..2 {
            let mut it = sub_line_iterator_helper::new(&mut file);
            // Iterate again and measure per-line and offsets
            let line = it.next().unwrap();
            let mut prev = line.offset;
            assert_eq!(prev, 0);
            for i in it.take(harness.lines - 1) {
                let (line, bol) = (i.line, i.offset);
                assert_eq!(bol - prev, harness.patt_len);
                assert_eq!(line, harness.patt);
                prev = bol;
            }
        }
    }


    #[test]
    fn test_iterator_from_offset_unindexed() {
        let (harness, mut file) =  Harness::new_small(100);

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let mut it = sub_line_iterator_helper::new_from(&mut file, offset);

        // Iterate again and verify we get the expected number of lines
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        assert_eq!(prev, harness.patt_len * harness.lines / 2);
        assert_eq!(line, harness.patt);

        let count = it.count() + 1;
        assert_eq!(count, harness.lines / 2);
    }

    #[test]
    fn test_iterator_middle_out() {
        let (harness, mut file) =  Harness::new_small(1000);
        let mut count = 0;

        // A few bytes after the middle of the file
        let offset = harness.patt_len * harness.lines / 2 + harness.patt_len / 2;
        let mut it = sub_line_iterator_helper::new_from(&mut file, offset);

        // Iterate forwards and backwards simultaneously
        let mut lineset = HashSet::new();
        loop {
            let mut done = true;
            if let Some(line) = it.next() {
                lineset.insert(line.offset);
                // We don't reach the end of the file
                assert!(line.offset < harness.lines * harness.patt_len);
                assert_eq!(line.line, harness.patt);
                count += 1;
                done = false;
            }
            if let Some(line) = it.next_back() {
                lineset.insert(line.offset);
                assert_eq!(line.line, harness.patt);
                count += 1;
                done = false;
            }
            if done {
                break;
            }
        }
        assert_eq!(harness.lines, lineset.len());
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_from_offset_indexed() {
        let (harness, mut file) =  Harness::new_small(100);
        let count = sub_line_iterator_helper::new(&mut file).count();
        assert_eq!(count, harness.lines);

        // A few bytes before the middle of the file
        let mut it = sub_line_iterator_helper::new_from(&mut file, harness.patt_len * harness.lines / 2 - harness.patt_len / 2);

        // Get first line and verify we get the expected position and line
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        let expected_offset = harness.patt_len * harness.lines / 2;
        assert_eq!(prev, expected_offset);
        assert_eq!(line, harness.patt);

        let count = it.count() + 1;
        assert_eq!(count, harness.lines / 2);
    }

    #[test]
    fn test_iterator_from_offset_start() {
        let (harness, mut file) =  Harness::new_small(100);
        let count = sub_line_iterator_helper::new_from(&mut file, 0).rev().count();
        assert_eq!(count, 0, "No lines iterable before offset 0");

        let count = sub_line_iterator_helper::new_from(&mut file, 1).rev().count();
        assert_eq!(count, 1, "First line is reachable from offset 1");

        let mut it = sub_line_iterator_helper::new_from(&mut file, 0);

        // Verify we see the first line
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        assert_eq!(prev, 0);
        assert_eq!(line, harness.patt);

        let count = it.count() + 1;
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_from_offset_end_of_file() {
        let (harness, mut file) =  Harness::new_small(100);
        let out_of_range = harness.patt_len * harness.lines;

        let count = sub_line_iterator_helper::new_from(&mut file, out_of_range).count();
        assert_eq!(count, 0, "No lines iterable after out-of-range");

        let count = sub_line_iterator_helper::new_from(&mut file, out_of_range).rev().count();
        assert_eq!(count, harness.lines, "Whole file is reached from end");

    }

    #[test]
    fn test_iterator_from_offset_out_of_range() {
        let (harness, mut file) =  Harness::new_small(100);

        let out_of_range = harness.patt_len * harness.lines + 2;

        let count = sub_line_iterator_helper::new_from(&mut file, out_of_range).rev().count();
        assert_eq!(count, harness.lines, "All lines iterable before out-of-range");

        let count = sub_line_iterator_helper::new_from(&mut file, out_of_range).count();
        assert_eq!(count, 0, "No lines iterable after out-of-range");
     }
}


// Tests for SubLineIterator
#[cfg(test)]
mod sub_line_wrap_tests {
    use std::collections::HashSet;
    use crate::sub_line_iterator_helper::Harness;
    use indexed_file::{LineViewMode, SubLineIterator, Log};


    fn wrapped_new(log: &mut Log, width: usize) -> SubLineIterator<Log> {
        let mode = LineViewMode::Wrap{width};
        SubLineIterator::new(log, mode)
    }

    fn wrapped_new_from(log: &mut Log, width: usize, offset: usize) -> SubLineIterator<Log> {
        let mode = LineViewMode::Wrap{width};
        SubLineIterator::new_from(log, mode, offset)
    }

    #[test]
    fn test_iterator() {
        let (harness, mut file) = Harness::default();
        let width = 10;
        let mut offset = 0;
        for i in wrapped_new(&mut file, width) {
            let (line, bol) = (i.line, i.offset);

            let expect_width = harness.expected_width(offset, width);
            assert_eq!(offset, bol);

            assert_eq!(line, harness.expected_line(offset, width));
            offset += expect_width;
        }
        assert_eq!(offset, harness.lines * harness.patt_len);
    }

    #[test]
    fn test_iterator_rev() {
        let (harness, mut file) = Harness::default();
        let width = 10;
        let mut offset = harness.lines * harness.patt_len;
        for i in wrapped_new(&mut file, width).rev() {
            let (line, bol) = (i.line, i.offset);
            let expect_width = harness.expected_width(bol, width);
            assert_eq!(offset, bol + expect_width);
            offset -= expect_width;

            assert_eq!(line, harness.expected_line(offset, width));
        }
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_iterator_rev_exhaust() {
        let (harness, mut file) = Harness::default();
        let width = 10;
        let expected_lines = harness.total_len(width);
        let actual = wrapped_new(&mut file, width).rev().count();
        assert_eq!(expected_lines, actual);
    }

    #[test]
    fn test_iterator_exhaust() {
        let (harness, mut file) = Harness::default();
        let width = 10;
        let expected_lines = harness.total_len(width);
        let actual = wrapped_new(&mut file, width).count();
        assert_eq!(expected_lines, actual);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        // Iterate start to end twice. Both passes should return expected lines/offsets.
        let (harness, mut file) = Harness::default();
        let width = 10;
        let expected_lines = harness.total_len(width);
        let actual = wrapped_new(&mut file, width).count();
        assert_eq!(expected_lines, actual);

        // Iterate again and measure per-line and offsets
        let mut offset = 0;
        for i in wrapped_new(&mut file, width) {
            let (line, bol) = (i.line, i.offset);

            let expect_width = harness.expected_width(offset, width);
            assert_eq!(offset, bol);

            let expect_line = harness.expected_line(offset, width);
            assert_eq!(line, expect_line);
            offset += expect_width;
        }
        assert_eq!(offset, harness.lines * harness.patt_len);
    }


    #[test]
    fn test_iterator_exhaust_half_and_twice() {
        // Iterate start to end twice. Both passes should return expected lines/offsets.
        let (harness, mut file) = Harness::default();
        let width = 10;
        let expected_lines = harness.total_len(width);
        let actual = wrapped_new(&mut file, width).take(expected_lines/2).count();
        assert_eq!(expected_lines/2, actual);

        for _ in 0..2 {
            // Iterate again and measure per-line and offsets
            let mut offset = 0;
            for i in wrapped_new(&mut file, width) {
                let (line, bol) = (i.line, i.offset);

                let expect_width = harness.expected_width(offset, width);
                assert_eq!(offset, bol);

                let expect_line = harness.expected_line(offset, width);
                assert_eq!(line, expect_line);
                offset += expect_width;
            }
            assert_eq!(offset, harness.lines * harness.patt_len);
        }
    }


    #[test]
    fn test_iterator_from_offset_unindexed() {
        let (harness, mut file) =  Harness::new_small(100);
        let width = 10;

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let mut it = wrapped_new_from(&mut file, width, offset);

        // Iterate verify we get the expected line
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        assert_eq!(prev, harness.expected_bol(offset, width));
        assert_eq!(line, harness.expected_line(offset, width));

        let count = it.count() + 1;
        assert_eq!(count, harness.total_len(width) / 2 + 2);
    }

    #[test]
    fn test_iterator_middle_out() {
        let (harness, mut file) =  Harness::new_small(100);
        let width = 10;

        // A few bytes after the middle of the file
        let offset = harness.patt_len * harness.lines / 2 + harness.patt_len / 2;
        let mut it = wrapped_new_from(&mut file, width, offset);

        let mut fwd_offset = harness.expected_bol(offset, width);
        let mut rev_offset = fwd_offset;

        // Iterate forwards and backwards simultaneously
        let mut lineset = HashSet::new();
        let mut count = 0;
        loop {
            let mut done = true;
            if let Some(line) = it.next() {
                lineset.insert(line.offset);
                // We don't reach the end of the file
                assert!(line.offset < harness.lines * harness.patt_len);
                assert_eq!(line.line, harness.expected_line(fwd_offset, width));
                fwd_offset += harness.expected_width(fwd_offset, width);
                count += 1;
                done = false;
            }
            if let Some(line) = it.next_back() {
                lineset.insert(line.offset);
                rev_offset -= harness.expected_width(rev_offset - 1, width);
                assert_eq!(line.offset, rev_offset);
                assert_eq!(line.line, harness.expected_line(rev_offset, width));
                count += 1;
                done = false;
            }
            if done {
                break;
            }
        }
        assert_eq!(harness.total_len(width), lineset.len());
        assert_eq!(count, harness.total_len(width));
    }

    #[test]
    fn test_iterator_from_offset_indexed() {
        let (harness, mut file) =  Harness::new_small(100);
        let width = 10;

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let mut it = wrapped_new_from(&mut file, width, offset);

        // Get first line and verify we get the expected position and line
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        let expected_offset = harness.expected_bol(offset, width);
        assert_eq!(prev, expected_offset);
        assert_eq!(line, harness.expected_line(offset, width));

        let count = it.count();
        assert_eq!(count, harness.total_len(width) / 2 + 1);
    }

    #[test]
    fn test_iterator_from_offset_start() {
        let (harness, mut file) =  Harness::new_small(100);
        let width = 10;

        let it = wrapped_new_from(&mut file, width, 0);
        let count = it.rev().count();
        assert_eq!(count, 0, "No lines iterable before offset 0");

        // FIXME: This is different from whole-line iteration.  We get the first line there when iterating backwards from 1.
        //        Should they act the same?
        let it = wrapped_new_from(&mut file, width, 1);
        let count = it.rev().count();
        assert_eq!(count, 0, "First line is not reachable from offset 1 in reverse");

        let it = wrapped_new_from(&mut file, width, width);
        let count = it.rev().count();
        assert_eq!(count, 1, "First line is reachable from offset 'width'");

        let mut it = wrapped_new_from(&mut file, width, 0);

        // Verify we see the first line
        let line = it.next().unwrap();
        let (line, offset) = (line.line, line.offset);

        assert_eq!(offset, 0);
        assert_eq!(line, harness.expected_line(offset, width));

        let count = it.count();
        assert_eq!(count, harness.total_len(width) - 1);
    }

    #[test]
    fn test_iterator_from_offset_end_of_file() {
        let (harness, mut file) =  Harness::new_small(100);
        let width = 10;

        let end_of_file = harness.patt_len * harness.lines;

        let it = wrapped_new_from(&mut file, width, end_of_file);
        let count = it.count();
        assert_eq!(count, 0, "No lines iterable after out-of-range");

        let it = wrapped_new_from(&mut file, width, end_of_file).rev();
        let count = it.count();
        assert_eq!(count, harness.total_len(width), "Whole file is reached from end");

        let it = wrapped_new_from(&mut file, width, end_of_file + 1).rev();
        let count = it.count();
        assert_eq!(count, harness.total_len(width), "Whole file is reached from way out-of-range");
    }

    #[test]
    fn test_iterator_from_offset_out_of_range() {
        let (harness, mut file) =  Harness::new_small(100);
        let width = 10;

        let out_of_range = harness.patt_len * harness.lines + 2;

        let it = wrapped_new_from(&mut file, width, out_of_range).rev();
        let count = it.count();
        assert_eq!(count, harness.total_len(width), "All lines iterable before out-of-range");

        let it = wrapped_new_from(&mut file, width, out_of_range);
        let count = it.count();
        assert_eq!(count, 0, "No lines iterable after out-of-range");
     }
}