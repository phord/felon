#[cfg(test)]
mod filtered_log_iterator_helper {
    use indexed_file::{IndexedLog, LineIndexerDataIterator, Log, LogStack};
    use indexed_file::files::{CursorLogFile, CursorUtil};

    pub(crate) struct Harness {
        pub(crate) patt_len: usize,
        pub(crate) lines: usize,
    }

    impl Harness {
        pub(crate) fn new(lines: usize) -> (Self, LogStack) {
            let patt_len = 9usize;
            let base = 10usize.pow(patt_len as u32 - 2);
            let buff = CursorLogFile::from_vec((base..base+lines).collect()).unwrap();
            let file = Log::from(buff);
            let file = LogStack::new(file);
            let s = Self {
                patt_len,
                lines,
            };
            (s, file)
        }


        // pub(crate) fn expected_line(&self, offset: usize, width: usize) -> &str {
        //     let offset = self.expected_bol(offset, width);
        //     let ofs = self.offset_into_line(offset);
        //     let width = self.expected_width(offset, width);
        //     &self.patt[ofs..ofs + width]
        // }

        pub(crate) fn default() -> (Self, LogStack) {
            Self::new(6000)
        }
    }

    pub(crate) fn new<LOG: IndexedLog>(log: &mut LOG) -> LineIndexerDataIterator<LOG> {
        LineIndexerDataIterator::new(log)
    }

}


// Tests for filtered_log iterators
#[cfg(test)]
mod filtered_log_iterator_tests {
    use std::collections::HashSet;

    use crate::filtered_log_iterator_helper::{new, Harness};
    use indexed_file::{IndexedLog, LineIndexerDataIterator};

    #[test]
    fn test_iterator() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        let mut it = new(&mut file);
        let line = it.next().unwrap();
        let (_line, prev) = (line.line, line.offset);
        let mut prev = prev;
        assert_eq!(prev, 0);
        let mut count = 1;
        for i in it {
            count += 1;
            assert!(count <= harness.lines);
            let (_line, bol) = (i.line, i.offset);
            assert_eq!(bol - prev, harness.patt_len);
            prev = bol;
        }
        assert_eq!(count, harness.lines);

        // Count again, but with the file already indexed
        let it = new(&mut file);
        assert_eq!(it.count(), harness.lines);
    }

    #[test]
    fn test_iterator_subset() {
        let (harness, mut file) = Harness::default();
        // Match lines from 5,000 to 5,999
        file.search_regex("5...$").unwrap();

        let mut it = new(&mut file);
        let line = it.next().unwrap();
        let (_line, prev) = (line.line, line.offset);
        let mut prev = prev;
        assert_eq!(prev, 5000 * harness.patt_len);
        for i in it.take(harness.lines - 1) {
            let (_line, bol) = (i.line, i.offset);
            assert_eq!(bol - prev, harness.patt_len);
            prev = bol;
        }

        let it = new(&mut file);
        let lines = it.map(|x| x.line).collect::<Vec<_>>();
        dbg!(&lines[..10]);

        let it = new(&mut file);
        assert_eq!(it.count(), 1000);
    }

    #[test]
    fn test_iterator_no_match() {
        let (_harness, mut file) = Harness::default();
        // Match lines from 5,000 to 5,999
        file.search_regex("xyz").unwrap();

        let mut it = new(&mut file);
        assert!(it.next().is_none());

        let it = new(&mut file);
        assert_eq!(it.count(), 0);
    }

    #[test]
    fn test_iterator_rev() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        let mut it = new(&mut file).rev();
        let line = it.next().unwrap();
        let (_line, prev) = (line.line, line.offset);
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        for i in it.take(harness.lines - 1) {
            let (_, bol) = (i.line, i.offset);
            // println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            prev = bol;
        }

        let it = new(&mut file);
        assert_eq!(it.count(), harness.lines);
    }


    #[test]
    fn test_iterator_gaps() {
        let (harness, mut file) = Harness::default();
        assert!(file.search_regex(r"0$").is_ok());

        let mut it = new(&mut file);
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);
        assert!(line.trim().ends_with("0"));
        let mut prev = prev;

        assert_eq!(prev, 0);

        let mut count = 1;
        for i in it.take(harness.lines - 1) {
            let (line, bol) = (i.line, i.offset);
            assert!(line.trim().ends_with("0"));
            assert_eq!(bol - prev, harness.patt_len * 10);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, harness.lines / 10);

        let it = new(&mut file);
        assert_eq!(it.count(), harness.lines / 10);
    }


    #[test]
    fn test_iterator_rev_gaps() {
        let (harness, mut file) = Harness::default();
        assert!(file.search_regex(r"0$").is_ok());

        let mut it = new(&mut file).rev();
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);
        assert!(line.trim().ends_with("0"));
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len * 10);

        let mut count = 1;
        for i in it.take(harness.lines - 1) {
            let (line, bol) = (i.line, i.offset);
            assert!(line.trim().ends_with("0"));
            assert_eq!(prev - bol, harness.patt_len * 10);
            prev = bol;
            count += 1;
        }
        assert_eq!(count, harness.lines / 10);

        let it = new(&mut file).rev();
        assert_eq!(it.count(), harness.lines / 10);
    }


    #[test]
    fn test_build_index() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();
        let mut it = new(&mut file);
        let line = it.next().unwrap();
        let prev = line.offset;
        let mut prev = prev;
        assert_eq!(prev, 0);
        for i in it.take(harness.lines - 1) {
            let bol = i.offset;
            assert_eq!(bol - prev, harness.patt_len);
            prev = bol;
        }
    }

    #[test]
    fn test_build_index_rev() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        let mut it = new(&mut file).rev();
        let line = it.next().unwrap();
        let (_line, prev) = (line.line, line.offset);
        let mut prev = prev;

        assert_eq!(prev, harness.lines * harness.patt_len - harness.patt_len);

        for i in it.take(harness.lines - 2) {
            let (_, bol) = (i.line, i.offset);
            // println!("{bol} {prev}");
            assert_eq!(prev - bol, harness.patt_len);
            // assert_eq!(line, harness.patt);
            prev = bol;
        }
    }


    #[test]
    fn test_iterator_from_offset_unindexed() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let range = offset..;
        let mut it = LineIndexerDataIterator::range(&mut file, &range);

        // Iterate again and verify we get the expected number of lines
        let line = it.next().unwrap();
        let (line, prev) = (line.line, line.offset);

        // Line returned is the line just before the middle of the file; the one that includes our offset
        assert!((prev..prev+line.len()).contains(&offset));
        // assert_eq!(line, patt);

        let count = it.count();
        assert_eq!(count, harness.lines / 2);
    }

    #[test]
    #[ignore]
    fn test_iterator_middle_out() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let range = offset..;
        let mut it = LineIndexerDataIterator::range(&mut file, &range);

        // Iterate forwards and backwards simultaneously
        let mut count = 0;
        let mut lineset = HashSet::new();
        loop {
            let mut done = true;
            if let Some(line) = it.next() {
                lineset.insert(line.offset);
                // We don't reach the end of the file
                assert!(line.offset < harness.lines * harness.patt_len);
                // assert_eq!(line.line, patt);
                count += 1;
                done = false;
            }
            if let Some(line) = it.next_back() {
                lineset.insert(line.offset);
                // assert_eq!(line.line, patt);
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
    fn test_iterator_timeout() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        {
            // Index half the lines.  Expect no timeout.
            let count = file.with_timeout(1000).iter().take(harness.lines / 2).count();
            assert_eq!(count, harness.lines / 2);
            assert!(!file.timed_out());
        }

        {
            // Set a timeout and then wait for it to pass.  Expect a timeout before we index more lines.
            let mut wrap = file.with_timeout(1);
            std::thread::sleep(std::time::Duration::from_millis(2));
            let count = wrap.iter().count();
            assert!(count < harness.lines);
            assert!(wrap.timed_out());

            // Timeout is persistent
            let count = wrap.iter().count();
            assert!(count < harness.lines);
            assert!(wrap.timed_out());
            drop(wrap);
            assert!(file.timed_out());
        }

        // After wrapper is dropped, file iterates normally
        let count = file.iter().count();
        assert_eq!(count, harness.lines);
        assert!(!file.timed_out());
    }

    #[test]
    fn test_iterator_from_offset_indexed() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        // Iterate whole file (indexed)
        let mut count = 0;
        for _ in LineIndexerDataIterator::new(&mut file) {
            count += 1;
        }
        assert_eq!(count, harness.lines);

        // A few bytes before the middle of the file
        let offset = harness.patt_len * harness.lines / 2 - harness.patt_len / 2;
        let range = offset..;
        let mut it = LineIndexerDataIterator::range(&mut file, &range);

        // Iterate again and verify we get the expected number of lines
        let line = it.next().unwrap();
        let (_, prev) = (line.line, line.offset);

        count = 1;
        assert_eq!(prev, harness.patt_len * (harness.lines / 2 - 1));
        // assert_eq!(line, patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, harness.lines / 2 + 1);
    }

    #[test]
    fn test_iterator_from_offset_start() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        // FIXME: These range checks should be run on both fwd and rev, both indexed and unindexed

        let mut count = 0;
        let range = ..0;
        for _ in LineIndexerDataIterator::range(&mut file, &range).rev() {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable before offset 0");

        count = 0;
        let range = ..1;
        for _line in LineIndexerDataIterator::range(&mut file, &range).rev() {
            count += 1;
        }
        assert_eq!(count, 1, "First line is reachable from offset 1");

        let range = 0..;
        let mut it = LineIndexerDataIterator::range(&mut file, &range);

        // Verify we see the first line
        let line = it.next().unwrap();
        let (_, prev) = (line.line, line.offset);

        count = 1;
        assert_eq!(prev, 0);
        // assert_eq!(line, patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, harness.lines);
    }

    #[test]
    fn test_iterator_from_offset_end_of_file() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        let out_of_range = harness.patt_len * harness.lines;

        let mut count = 0;
        for _ in LineIndexerDataIterator::range(&mut file,&( out_of_range..)) {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable after out-of-range");

        for _ in LineIndexerDataIterator::range(&mut file, &(..out_of_range)).rev() {
            count += 1;
        }
        assert_eq!(count, harness.lines, "Whole file is reached from end");

    }

    #[test]
    fn test_iterator_from_offset_out_of_range() {
        let (harness, mut file) = Harness::default();
        file.search_regex("000").unwrap();

        let out_of_range = harness.patt_len * harness.lines + 2;

        let mut count = 0;
        for _line in LineIndexerDataIterator::range(&mut file, &(..out_of_range)).rev() {
            count += 1;
        }
        assert_eq!(count, harness.lines, "All lines iterable before out-of-range");

        count = 0;
        for _ in LineIndexerDataIterator::range(&mut file, &(out_of_range..)) {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable after out-of-range");
     }


     #[test]
     fn test_filtered_gap_filler() {
        let (harness, mut file) = Harness::default();
        file.search_regex("00$").unwrap();

        let pos = file.seek(0);
        file.resolve_gaps(&pos);

        // First stats block is for the base log
        assert_eq!(file.info().next().unwrap().lines_indexed, harness.lines);

        // Second stats block is for the filtered log
        assert_eq!(file.info().nth(1).unwrap().lines_indexed, harness.lines / 100);
     }

     #[test]
     fn test_filtered_gap_filler_partial() {
        // Resolve gaps when some of the gaps are already resolved
        let (harness, mut file) = Harness::default();
        file.search_regex("0$").unwrap();

        let size = harness.lines * harness.patt_len;
        let offsets = (0..harness.lines/100).map(|_| rand::random::<usize>() % size).collect::<Vec<_>>();

        // In case of failure, copy the random vector from the log here and uncomment this line to continue debugging:
        // let offsets = vec![30431, 47575, 3525, 185, 44166, 41886, 34670, 21278, 33469, 28364, 42469, 14464, 11461, 52506, 21765, 17043, 22367, 18331, 42082, 46408, 5961, 9943, 50902, 6684, 37820, 17028, 35756, 47341, 46853, 50429, 7729, 21521, 46755, 49777, 16002, 1483, 7347, 4243, 4860, 20703, 48702, 12057, 42099, 2624, 15159, 47419, 1596, 4940, 51691, 12911, 27690, 25517, 39068, 53378, 13010, 4652, 2462, 44391, 2575, 21026];
        println!("let offsets = vec!{:?};", offsets);

        // read some random lines to start
        for offset in offsets {
            let _l = LineIndexerDataIterator::range(&mut file, &(offset..)).next();
            // println!("Read line at offset {}: {:?}", offset, l);
        }

        // Now resolve the remaining gaps.  A single call should resolve all remaining gaps.
        let pos = file.seek(0);
        file.resolve_gaps(&pos);

        // Verify indexes are complete
        // First stats block is for the base log
        assert_eq!(file.info().next().unwrap().lines_indexed, harness.lines);

        // Second stats block is for the filtered log
        assert_eq!(file.info().nth(1).unwrap().lines_indexed, harness.lines / 10);

        // Dump the index
        // let mut pos = file.seek(0);
        // for _ in 0..1000 {
        //     match file.next(&pos) {
        //         GetLine::Hit(p, line) => {
        //             println!(" HIT: {:?} {:?}", p, line);
        //             pos = p
        //         },
        //         GetLine::Miss(p) => {
        //             println!("MISS: {:?}", p);
        //             pos = p;
        //         },
        //         GetLine::Timeout(p) => {
        //             println!("Timeout {:?}", p);
        //             break;
        //         },
        //     }
        //     if pos.is_virtual(){
        //         break;
        //     }
        // }

        let mut prev = 0;
        for line in file.iter_lines() {
            // println!("{} {:?}", line.offset - prev, line);
            if prev > 0 {
                assert_eq!(line.offset - prev, harness.patt_len * 10);
            }
            prev = line.offset;
        }

        // First stats block is for the base log
        assert_eq!(file.info().next().unwrap().lines_indexed, harness.lines);

        // Second stats block is for the filtered log
        assert_eq!(file.info().nth(1).unwrap().lines_indexed, harness.lines / 10);
     }

}
