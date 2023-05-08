// use indexed_file::indexer::line_indexer::LineIndexer;
// use indexed_file::files::{LogSource, TextLogFile, new_text_file};

use std::io::Cursor;
use std::io::Write;
use super::{Stream, LogFile};

pub type CursorLogFile = std::io::Cursor<Vec<u8>>;

impl Stream for CursorLogFile {
    fn get_length(&self) -> usize {
        self.get_ref().len()
    }

    // Wait on any data at all; Returns true if file is still open
    fn wait(&mut self) -> bool { false }
}

impl LogFile for CursorLogFile { }

pub trait CursorUtil {
    // fn truncate(self) -> CursorLogFile;
    fn from_vec<T: std::fmt::Display>(data: Vec<T>) -> std::io::Result<CursorLogFile>;
}

impl CursorUtil for CursorLogFile {
    // // Truncate CursorLogFile at current position
    //  use std::io::Seek;
    //  use std::io::SeekFrom;
    // fn truncate(self) -> CursorLogFile {
    //     let mut curs = self;
    //     let pos = curs.seek(SeekFrom::Current(0)).unwrap() as usize;
    //     let inner:Vec<u8> = curs.into_inner().iter().take(pos).cloned().collect();
    //     Cursor::new(inner)
    // }

    fn from_vec<T: std::fmt::Display>(data: Vec<T>) -> std::io::Result<CursorLogFile> {
        let mut c = Cursor::new(vec![]);
        for t in data {
            writeln!(c, "{t}")?;
        }
        Ok(c)
    }
}

#[test]
fn mock_cursor() {
    let lines = 50;
    use crate::Log;
    let buff = CursorLogFile::from_vec((0..lines).into_iter().collect()).unwrap();
    let mut index = Log::from(buff);
    for line in index.iter_lines() {
        print!("{}: {line}", line.offset);
    }
    println!();
    assert_eq!(lines, index.iter_lines().count());
}
