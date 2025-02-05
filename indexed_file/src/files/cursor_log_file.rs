// CursorLogFile is an in-memory file that can be used as a LogFile.  It's used in tests to work on ephemeral files.

use std::io::Cursor;
use std::io::Seek;
use std::io::Write;

use super::Stream;

pub type CursorLogFile = std::io::Cursor<Vec<u8>>;

impl Stream for CursorLogFile {
    fn len(&self) -> usize {
        self.get_ref().len()
    }

    fn poll(&mut self, _timeout: Option<std::time::Instant>) -> usize { self.len() }

    fn is_open(&self) -> bool {
        false
    }
}

pub trait CursorUtil {
    fn from_vec<T: std::fmt::Display>(data: Vec<T>) -> std::io::Result<CursorLogFile>;
}

impl CursorUtil for CursorLogFile {
    fn from_vec<T: std::fmt::Display>(data: Vec<T>) -> std::io::Result<CursorLogFile> {
        let mut c = Cursor::new(vec![]);
        for t in data {
            writeln!(c, "{t}")?;
        }
        c.seek(std::io::SeekFrom::Start(0))?;
        Ok(c)
    }
}

#[cfg(test)]
use crate::IndexedLog;

#[test]
fn mock_cursor() {
    let lines = 50;
    use crate::Log;
    let buff = CursorLogFile::from_vec((0..lines).collect()).unwrap();
    let mut index = Log::from(buff);
    for line in index.iter_lines() {
        print!("{}: {line}", line.offset);
    }
    println!();
    assert_eq!(lines, index.iter_lines().count());
}
