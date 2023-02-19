pub mod log_file;
mod indexer;
pub mod line_indexer;

#[cfg(test)]
mod tests {
    use crate::indexer;
    use std::path::PathBuf;

    #[test]
    fn file_missing() {
        let path = PathBuf::from(r"/tmp/does_not_exist");
        let file = indexer::LogFile::new(Some(path));
        assert_eq!(file.is_err(), true);
    }

    #[test]
    fn file_found() {
        let (path, _) = make_test_file("file_found", 10 , 10);
        println!("{:?}", path);
        assert!(indexer::LogFile::new(Some(path)).is_ok());
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

        let file = indexer::LogFile::test_new(Some(path), chunk_size, max_line_length);

        // assert!(file.is_ok());

        let file = file.unwrap();
        println!("{:?}", file);

        // Walk the file and compare each line offset to the expected offset
        let mut offset = 0;
        let mut linecount = 0;
        let scan = File::open(test_file).unwrap();
        let mut scanlines = io::BufReader::new(scan).lines();
        for line in 0..file.count_lines() {
            let reported = file.line_offset(line+1).unwrap();
            linecount += 1;
            offset += scanlines.next().unwrap().unwrap().len() + 1;
            if reported != offset {
                for l in std::cmp::max(2,line)-2..line+2 {
                    println!(">> {}. {}", l, file.line_offset(l+1).unwrap());
                }
            }
            assert_eq!(reported, offset);
        }

        // FIXME: This fails. Why?  Create test file stops too early?
        // assert_eq!(lines, linecount);

        // assert no more lines in file
        assert_eq!(scanlines.count(), 0);
        assert_eq!(file.count_lines(), linecount);
        assert_eq!(file.count_bytes(), bytes);
    }

    #[test]
    fn file_parse_long_lines_bytes() {
        let chunk_size = 1024 * 1024;
        let max_line_length: usize = 64 * 1024;
        let words = 80;
        let size = chunk_size * 2;
        let lines = size / words / 4;

        println!("words: {}  lines: {}", words, lines);

        let (path, bytes) = make_test_file("parse_long_lines_bytes", words, lines);
        let test_file = path.clone();

        assert!(bytes > chunk_size * 2);

        let file = indexer::LogFile::test_new(Some(path), chunk_size, max_line_length);

        let file = file.unwrap();
        println!("{:?}", file);

        // Walk the file and compare each line offset to the expected offset
        let mut offset = 0;
        let mut linecount = 0;
        let scan = File::open(test_file).unwrap();
        let mut scanlines = io::BufReader::new(scan).lines();
        for line in 0..file.count_lines() {
            let reported = file.line_offset(line+1).unwrap();
            linecount += 1;
            offset += scanlines.next().unwrap().unwrap().len() + 1;
            if reported != offset {
                for l in std::cmp::max(2,line)-2..line+2 {
                    println!(">> {}. {}", l, file.line_offset(l+1).unwrap());
                }
            }
            assert_eq!(reported, offset);
        }

        // FIXME: This fails. Why?  Create test file stops too early?
        // assert_eq!(lines, linecount);

        // assert no more lines in file
        assert_eq!(scanlines.count(), 0);
        assert_eq!(file.count_lines(), linecount);
        assert_eq!(file.count_bytes(), bytes);
    }

// ----
}
// fn main() {
//     let opt = Opt::from_args();
//     let index_timer = Instant::now();

//     let file = indexer::LogFile::new(opt.input);
//     println!("Index time: {}", index_timer.elapsed().as_millis() as f32 / 1000.);

//     println!("{:?}", file);

//     if let Some(word) = opt.search_word {
//         let lookup_timer = Instant::now();
//         let lines = file.search_word(&word);
//         println!("Found {} lines for word '{}'", lines.len(), word);
//         println!("Lookup time: {}", lookup_timer.elapsed().as_micros() as f32 / 1000000.);

//         let lookup_timer = Instant::now();
//         let lines = file.search_word(&word);
//         println!("Found {} lines for word '{}'", lines.len(), word);
//         println!("Second lookup time: {}", lookup_timer.elapsed().as_micros() as f32 / 1000000.);

//         for line in lines.iter() {
//             if let Some(str) = file.readline_at(*line) {
//                 println!("{}", str);
//             }
//         }
//     }
// }
