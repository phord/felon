// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

#[cfg(test)]
mod sub_line_iterator_helper {
    use indexed_file::{LineViewMode, SubLineIterator, Log};
    use indexed_file::files::new_mock_file;

    pub(crate) struct Harness {
        pub(crate) patt: String,
        pub(crate) patt_len: usize,
        pub(crate) lines: usize,
        pub(crate) file: Log,
    }

    impl Harness {
        pub(crate) fn new(patt: &str, lines: usize, ) -> Self {
            let patt_len = patt.len();
            let file = new_mock_file(patt, patt_len * lines, 100);
            let file = Log::from(file);
            Self {
                patt: patt.to_string(),
                patt_len,
                lines,
                file,
            }
        }

        pub(crate) fn default() -> Self {
            Self::new("abcdefghijklmnopqrstuvwxyz\n", 6000)
        }

        pub(crate) fn new_small(lines: usize) -> Self {
            Self::new("abcdefghijklmnopqrstuvwxyz\n", lines)
        }
    }

    pub(crate) fn new(log: &mut Log) -> SubLineIterator {
        let mode = LineViewMode::WholeLine;
        SubLineIterator::new(log, mode)
    }

    pub(crate) fn new_from(log: &mut Log, offset: usize) -> SubLineIterator {
    let mode = LineViewMode::WholeLine;
        SubLineIterator::new_from(log, mode, offset)
    }
}

// Tests for LineIndexerIterator
#[cfg(test)]
mod logfile_iterator_tests {
    use crate::sub_line_iterator_helper::Harness;

    #[test]
    fn test_iterator() {
        let mut harness = Harness::default();
        let mut it = harness.file.iter_offsets();
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
        let mut harness = Harness::default();
        let mut it = harness.file.iter_offsets().rev();
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
        let mut harness = Harness::default();
        let mut it = harness.file.iter_offsets().rev();
        let prev = it.next().unwrap();
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        let mut count = 1;
        for i in it {
            let bol = i;
            println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_fwd_rev_meet() {
        let mut harness = Harness::new_small(10);
        let mut it = harness.file.iter_offsets();
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
        let mut harness = Harness::default();
        let mut count = 0;
        for _ in harness.file.iter_offsets() {
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        let mut harness = Harness::default();
        let mut count = 0;
        for _ in harness.file.iter_offsets() {
            count += 1;
        }
        assert_eq!(count, harness.lines);

        let mut it = harness.file.iter_offsets();
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
        let mut harness = Harness::default();
        let mut count = 0;
        for _ in harness.file.iter_offsets().take(harness.lines/2) {
            count += 1;
        }
        assert_eq!(count, harness.lines/2);

        for _ in 0..2 {
            let mut it = harness.file.iter_offsets();
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
        let mut harness = Harness::default();
        let mut it = sub_line_iterator_helper::new(&mut harness.file);
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
        let mut harness = Harness::default();
        let mut it = sub_line_iterator_helper::new(&mut harness.file).rev();
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);
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
        let mut harness = Harness::new_small(3);
        let mut it = sub_line_iterator_helper::new(&mut harness.file).rev();
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

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
        let mut harness = Harness::default();
        let mut count = 0;
        for _ in sub_line_iterator_helper::new(&mut harness.file) {
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_exhaust_twice() {
        let mut harness = Harness::default();
        let mut count = 0;
        for _ in sub_line_iterator_helper::new(&mut harness.file) {
            count += 1;
        }
        assert_eq!(count, harness.lines);

        let mut it = sub_line_iterator_helper::new(&mut harness.file);
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
        let mut harness = Harness::default();
        let mut count = 0;
        for _ in sub_line_iterator_helper::new(&mut harness.file).take(harness.lines/2) {
            count += 1;
        }
        assert_eq!(count, harness.lines/2);

        for _ in 0..2 {
            let mut it = sub_line_iterator_helper::new(&mut harness.file);
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
        let mut harness = Harness::new_small(100);

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let mut it = sub_line_iterator_helper::new_from(&mut harness.file, offset);

        // Iterate again and verify we get the expected number of lines
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        let mut count = 1;
        assert_eq!(prev, harness.patt_len * harness.lines / 2);
        assert_eq!(line, harness.patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, harness.lines / 2);
    }

    #[test]
    fn test_iterator_middle_out() {
        let mut harness = Harness::new_small(1000);
        let mut count = 0;

        // A few bytes after the middle of the file
        let offset = harness.patt_len * harness.lines / 2 + harness.patt_len / 2;
        let mut it = sub_line_iterator_helper::new_from(&mut harness.file, offset);

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
        let mut harness = Harness::new_small(100);
        let mut count = 0;
        for _ in sub_line_iterator_helper::new(&mut harness.file) {
            count += 1;
        }
        assert_eq!(count, harness.lines);

        // A few bytes before the middle of the file
        let mut it = sub_line_iterator_helper::new_from(&mut harness.file, harness.patt_len * harness.lines / 2 - harness.patt_len / 2);

        // Get first line and verify we get the expected position and line
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        let expected_offset = harness.patt_len * harness.lines / 2;
        assert_eq!(prev, expected_offset);
        assert_eq!(line, harness.patt);

        count = 1;
        for _ in it {
            count += 1;
        }
        assert_eq!(count, harness.lines / 2);
    }

    #[test]
    fn test_iterator_from_offset_start() {
        let mut harness = Harness::new_small(100);
        let mut count = 0;
        for _ in sub_line_iterator_helper::new_from(&mut harness.file, 0).rev() {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable before offset 0");

        for _ in sub_line_iterator_helper::new_from(&mut harness.file, 1).rev() {
            count += 1;
        }
        assert_eq!(count, 1, "First line is reachable from offset 1");

        let mut it = sub_line_iterator_helper::new_from(&mut harness.file, 0);

        // Verify we see the first line
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        count = 1;
        assert_eq!(prev, 0);
        assert_eq!(line, harness.patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }
    #[test]
    fn test_iterator_from_offset_end_of_file() {
        let mut harness = Harness::new_small(100);
        let out_of_range = harness.patt_len * harness.lines;

        let mut count = 0;
        for _ in sub_line_iterator_helper::new_from(&mut harness.file, out_of_range) {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable after out-of-range");

        for _ in sub_line_iterator_helper::new_from(&mut harness.file, out_of_range).rev() {
            count += 1;
        }
        assert_eq!(count, harness.lines, "Whole file is reached from end");

    }

    #[test]
    fn test_iterator_from_offset_out_of_range() {
        let mut harness = Harness::new_small(100);

        // Length + 1 is ok.  Whole file is iterated.  Length + 2 is "out of range".
        let out_of_range = harness.patt_len * harness.lines + 2;

        let mut count = 0;
        for _ in sub_line_iterator_helper::new_from(&mut harness.file, out_of_range).rev() {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable before out-of-range");

        count = 0;
        for _ in sub_line_iterator_helper::new_from(&mut harness.file, out_of_range) {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable after out-of-range");
     }
}