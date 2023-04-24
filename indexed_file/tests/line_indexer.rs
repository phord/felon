// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

// Tests for LineIndexerIterator
#[cfg(test)]
mod logfile_iterator_tests {
    use indexed_file::files::new_mock_file;
    use indexed_file::indexer::LineIndexer;

    #[test]
    fn test_iterator() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 6000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut it = file.iter_offsets();
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
        let mut it = file.iter_offsets().rev();
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
        let mut it = file.iter_offsets().rev();
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
        let mut it = file.iter_offsets();
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
        for _ in file.iter_offsets() {
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
        for _ in file.iter_offsets() {
            count += 1;
        }
        assert_eq!(count, lines + 1);

        let mut it = file.iter_offsets();
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
        for _ in file.iter_offsets().take(lines/2) {
            count += 1;
        }
        assert_eq!(count, lines/2);

        for _ in 0..2 {
            let mut it = file.iter_offsets();
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
    use std::collections::HashSet;

    use indexed_file::files::new_mock_file;
    use indexed_file::indexer::LineIndexer;

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


    #[test]
    fn test_iterator_from_offset_unindexed() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut count = 0;

        // A few bytes after the middle of the file
        let mut it = file.iter_lines_from(patt_len * lines / 2 - patt_len / 2);

        // Iterate again and verify we get the expected number of lines
        let (line, prev) = it.next().unwrap();
        assert_eq!(prev, patt_len * lines / 2);
        assert_eq!(line, patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, lines / 2);
    }

    #[test]
    fn test_iterator_middle_out() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 1000;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut count = 0;

        // A few bytes after the middle of the file
        let mut it = file.iter_lines_from(patt_len * lines / 2 - patt_len / 2);

        // Iterate forwards and backwards simultaneously
        let mut lineset = HashSet::new();
        loop {
            let mut done = true;
            if let Some((line, offset)) = it.next() {
                lineset.insert(offset);
                if offset != lines * patt_len {
                    assert_eq!(line, patt);
                } else {
                    // Last "line" in the file is empty
                    assert!(line.is_empty());
                }
                count += 1;
                done = false;
            }
            if let Some((line, offset)) = it.next_back() {
                lineset.insert(offset);
                assert_eq!(line, patt);
                count += 1;
                done = false;
            }
            if done {
                break;
            }
        }
        assert_eq!(lines + 1, lineset.len());
        assert_eq!(count, lines + 1);
    }

    #[test]
    fn test_iterator_from_offset_indexed() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut count = 0;
        for _ in file.iter_lines() {
            count += 1;
        }
        assert_eq!(count, lines + 1);

        count = 0;

        // A few bytes before the middle of the file
        let mut it = file.iter_lines_from(patt_len * lines / 2 - patt_len / 2);

        // Iterate again and verify we get the expected number of lines
        let (line, prev) = it.next().unwrap();
        assert_eq!(prev, patt_len * lines / 2);
        assert_eq!(line, patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, lines / 2);
    }

    #[test]
    fn test_iterator_from_offset_start() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let mut count = 0;
        for _ in file.iter_lines_from(0).rev() {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable before offset 0");

        for _ in file.iter_lines_from(1).rev() {
            count += 1;
        }
        assert_eq!(count, 1, "First line is reachable from offset 1");

        count = 0;

        let mut it = file.iter_lines_from(0);

        // Verify we see all but the first line
        let (line, prev) = it.next().unwrap();
        assert_eq!(prev, patt_len);
        assert_eq!(line, patt);

        for _ in it {
            count += 1;
        }
        assert_eq!(count, lines - 1);
    }
    #[test]
    fn test_iterator_from_offset_end_of_file() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);
        let out_of_range = patt_len * lines;

        let mut count = 0;
        for _ in file.iter_lines_from(out_of_range) {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable after out-of-range");

        for _ in file.iter_lines_from(out_of_range).rev() {
            count += 1;
        }
        assert_eq!(count, lines, "Whole file is reached from end");

    }

    #[test]
    fn test_iterator_from_offset_out_of_range() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let file = new_mock_file(patt, patt_len * lines, 100);
        let mut file = LineIndexer::new(file);

        // Length + 1 is ok.  Whole file is iterated.  Length + 2 is "out of range".
        let out_of_range = patt_len * lines + 2;

        let mut count = 0;
        for _ in file.iter_lines_from(out_of_range).rev() {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable before out-of-range");

        count = 0;
        for _ in file.iter_lines_from(out_of_range) {
            count += 1;
        }
        assert_eq!(count, 0, "No lines iterable after out-of-range");
     }
}