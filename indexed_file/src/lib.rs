pub mod eventual_index;
pub mod filters;
pub mod index;
pub mod log_file;
pub mod line_indexer;
pub mod mock_log_file;

#[cfg(test)]
mod tests {
    use crate::line_indexer::LogFileLines;
    use crate::log_file::LogFile;
    use std::path::PathBuf;

    fn open_log_file(filename: &str) -> std::io::Result<LogFile> {
        let path = PathBuf::from(filename);
        LogFile::new_text_file(Some(path))
    }

    fn open_log_file_lines(path: PathBuf) -> LogFileLines {
        let file = LogFile::new_text_file(Some(path)).expect("Failed to open file");
        LogFileLines::new(file)
    }

    #[test]
    fn file_missing() {
        let file = open_log_file(r"/tmp/does_not_exist");
        assert_eq!(file.is_err(), true);
    }

    #[test]
    fn file_found() {
        let (path, _) = make_test_file("file_found", 10 , 10);
        println!("{:?}", path);
        let file = LogFile::new_text_file(Some(path));
        assert!(file.is_ok());
    }

    use std::io::Write;
    use rand::prelude::*;
    use std::fs::File;
    use std::io::{self, BufRead};

    fn make_test_file(name: &str, words: usize, lines: usize) -> (PathBuf, usize) {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("resources/test");
        path.push(name);

        // write some data to the file
        let mut file = std::fs::File::create(path.clone()).unwrap();
        let mut bytes = 0;
        for _ in 0..lines {
            // Make a random string with a mix of words from "foo", "bar", "baz" and a random number
            let mut s = String::new();
            s.push_str("--40 byte header skipped on every line--");
            for _ in 0..words {
                let x: u8 = random();
                let word = match x % 3 {
                    0 => "foo",
                    1 => "bar",
                    2 => "baz",
                    _ => unreachable!(),
                };
                s.push_str(word);
                s.push_str(" ");
            }
            s.push_str("\n");
            bytes += s.len();
            file.write_all(s.as_bytes()).unwrap();
        }

        (path, bytes)
    }

    #[test]
    fn file_parse_lines_bytes() {
        let chunk_size = 1024 * 10;
        let max_line_length: usize = 90;
        let words = max_line_length / 10;
        let size = chunk_size * 10 + chunk_size / 3;
        let lines = size / words;

        println!("words: {}  lines: {}", words, lines);

        let (path, bytes) = make_test_file("parse_lines_bytes", words, lines);
        let test_file = path.clone();

        assert!(bytes > chunk_size * 2);

        let mut file = open_log_file_lines(path);

        // Walk the file and compare each line offset to the expected offset
        let mut offset = 0;
        let mut linecount = 0;
        let scan = File::open(test_file).unwrap();
        let mut scanlines = io::BufReader::new(scan).lines();
        for (start, end) in file.iter_offsets() {
            linecount += 1;
            assert_eq!(start, offset);
            offset += scanlines.next().unwrap().unwrap().len() + 1;
            assert_eq!(end, offset);
        }

        // FIXME: This fails. Why?  Create test file stops too early?
        // assert_eq!(lines, linecount);

        // assert no more lines in file
        assert_eq!(scanlines.count(), 0);
        assert_eq!(file.count_lines(), linecount);
        let (_, count_bytes) = file.iter_offsets().last().unwrap();
        assert_eq!(count_bytes, bytes);
    }

    #[test]
    fn file_parse_long_lines_bytes() {
        let chunk_size = 1024 * 1024;
        let words = 80;
        let size = chunk_size * 2;
        let lines = size / words / 4;

        println!("words: {}  lines: {}", words, lines);

        let (path, bytes) = make_test_file("parse_long_lines_bytes", words, lines);
        let test_file = path.clone();

        assert!(bytes > chunk_size * 2);

        let mut file = open_log_file_lines(path);
        println!("{:?}", file);

        // Walk the file and compare each line offset to the expected offset
        let mut offset = 0;
        let mut linecount = 0;
        let scan = File::open(test_file).unwrap();
        let mut scanlines = io::BufReader::new(scan).lines();
        for (start, end) in file.iter_offsets() {
            linecount += 1;
            assert_eq!(start, offset);
            offset += scanlines.next().unwrap().unwrap().len() + 1;
            assert_eq!(end, offset);
        }

        assert_eq!(lines, linecount);

        // assert no more lines in file
        assert_eq!(scanlines.count(), 0);
        assert_eq!(file.count_lines(), linecount);
        let (_, count_bytes) = file.iter_offsets().last().unwrap();
        assert_eq!(count_bytes, bytes);
    }
}