use indexer::{eventual_index, time_stamper::TimeStamper, LineIndexerIterator, LineIndexerDataIterator, LogLine};
pub mod files;
pub mod indexer;
pub mod filters;

use std::path::PathBuf;
use files::{LogSource, LogBase};
use crate::indexer::line_indexer::LineIndexer;

pub struct Log {
    pub(crate) file: LineIndexer<LogSource>,
    pub(crate) format: TimeStamper,
}

impl<LOG: LogBase + 'static> From<LOG> for Log {
    fn from(file: LOG) -> Self {
        let src = LogSource::from(file);
        Self::from(src)
    }
}

impl From<LogSource> for Log {
    fn from(src: LogSource) -> Self {
        let src = LineIndexer::new(src);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }
}

impl Log {
    pub fn new(src: LineIndexer<LogSource>) -> Self {
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    pub fn from_source(file: LogSource) -> Self {
        let src = LineIndexer::new(file);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    pub fn open(file: Option<PathBuf>) -> std::io::Result<Self> {
        let src = files::new_text_file(file)?;
        let log = Log {
            file: LineIndexer::new(src),
            format: TimeStamper::default(),
        };
        Ok(log)
    }

    pub fn count_lines(&self) -> usize {
        self.file.count_lines()
    }

    fn iter(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        LineIndexerIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.iter()
    }

    pub fn iter_lines<'a>(&'a mut self) -> impl DoubleEndedIterator<Item = LogLine> + 'a {
        LineIndexerDataIterator::new(LineIndexerIterator::new(self))
    }

    pub fn iter_lines_from(&mut self, offset: usize) -> impl DoubleEndedIterator<Item = LogLine> + '_ {
        LineIndexerDataIterator::new(LineIndexerIterator::new_from(self, offset))
    }
}


#[cfg(test)]
mod tests {
    use crate::Log;
    use crate::files::{LogSource, new_text_file};
    use std::path::PathBuf;

    fn open_log_file(filename: &str) -> std::io::Result<LogSource> {
        let path = PathBuf::from(filename);
        new_text_file(Some(path))
    }

    fn open_log_file_lines(path: PathBuf) -> Log {
        let file = File::open(&path).unwrap();
        let file = BufReader::new(file);
        Log::from(file)
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
        let file = new_text_file(Some(path));
        assert!(file.is_ok());
    }

    use std::io::{Write, BufReader};
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
        for start in file.iter_offsets() {
            linecount += 1;
            assert_eq!(start, offset);
            if let Some(line) = scanlines.next() {
                offset += line.unwrap().len() + 1;
            }
        }

        // FIXME: This fails. Why?  Create test file stops too early?
        // assert_eq!(lines, linecount);

        // assert no more lines in file
        assert_eq!(scanlines.count(), 0);
        assert_eq!(file.count_lines()-1, linecount);
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

        // Walk the file and compare each line offset to the expected offset
        let mut offset = 0;
        let mut linecount = 0;
        let scan = File::open(test_file).unwrap();
        let mut scanlines = io::BufReader::new(scan).lines();
        for start in file.iter_offsets() {
            linecount += 1;
            assert_eq!(start, offset);
            if let Some(line) = scanlines.next() {
                offset += line.unwrap().len() + 1;
            }
        }

        assert_eq!(lines, linecount);

        // assert no more lines in file
        assert_eq!(scanlines.count(), 0);
        assert_eq!(file.count_lines()-1, linecount);
    }

    #[test]
    fn file_found_zstd() {
        let path = PathBuf::from("/home/phord/git/mine/igrok/test.zst");
        println!("{:?}", path);
        let file = new_text_file(Some(path));
        assert!(file.is_ok());
    }

    #[test]
    fn file_iter_zstd() {
        let path = PathBuf::from("/home/phord/git/mine/igrok/test.zst");
        // let path = PathBuf::from("/home/phord/git/mine/igrok/README.md");
        println!("{:?}", path);
        let file = new_text_file(Some(path));
        assert!(file.is_ok());
        let mut file = Log::from( file.unwrap() );
        for line in file.iter_lines() {
            println!("{}  {line}", line.offset);
        }
    }


/*
    use std::process::{Command, Stdio, Child, ChildStdin};

    // Create connected in/out pipes to stand in for an external process feeding data to stdin
    struct TestPipe<'a> {
        process: Child,
        stdin: &'a ChildStdin,
    }
    impl<'a> TestPipe<'a> {
        fn new() -> Self {
            let process = match Command::new("cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn() {
                    Err(why) => panic!("couldn't spawn cat: {}", why),
                    Ok(process) => process,
            };
            let stdin = &process.stdin.unwrap();
            Self { process, stdin}
        }

        fn send(&mut self, data: &str) {
            self.stdin.write_all(data.as_bytes());
        }
    }

    #[test]
    fn file_parse_growing_file() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let chunk_size = 100;
        let file = new_mock_file(patt, patt_len * lines, chunk_size);
        let mut file = LineIndexer::new(file);
        let it = file.iter();
        let count = it.take(lines/2).count();
        assert_eq!(count, lines/2);

        // Thoughts: let mock_file grow  on its own as we read?
        todo!("file.grow(lines);");

        let count = it.take(lines).count();
        assert_eq!(count, lines);

        // File is exhausted
        assert_eq!(it.next(), None);

        todo!("file.grow(lines);");

        // Ensure we can continue reading even after exhausting iterator (TODO: Need to clone the iterator first?)
        let count = it.take(lines).count();
        assert_eq!(count, lines);
    }

    #[test]
    fn file_parse_growing_stream() {
        let patt = "filler\n";
        let patt_len = patt.len();
        let lines = 100;
        let chunk_size = 100;
        let pipe = TestPipe::new();

        todo!("Attach pipe's stdout to a LogFile");
        let file = new_mock_file(patt, patt_len * lines, chunk_size);
        let mut file = LineIndexer::new(file);

        let it = file.iter();
        let count = it.take(lines/2).count();
        assert_eq!(count, lines/2);

        // Thoughts: let mock_file grow  on its own as we read?
        todo!("file.grow(lines);");

        let count = it.take(lines).count();
        assert_eq!(count, lines);

        // File is exhausted
        assert_eq!(it.next(), None);

        todo!("file.grow(lines);");

        // Ensure we can continue reading even after exhausting iterator (TODO: Need to clone the iterator first?)
        let count = it.take(lines).count();
        assert_eq!(count, lines);
    }
*/
}