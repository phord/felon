/// SaneLines combines a SaneIndex with a LogFile to provide an iterator over lines in a log file.

use crate::{indexer::{sane_index::SaneIndex, sane_indexer::SaneIndexer}, Log, LogLine};

use super::IndexedLog;

pub struct SaneLines<'a, R> {
    indexer: &'a mut R,
}

impl<'a, R: IndexedLog> SaneLines<'a, R> {
    pub fn new(indexer: &'a mut R) -> Self {
        SaneLines {
            indexer,
        }
    }
}

impl<'a, R: IndexedLog> Iterator for SaneLines<'a, R> {
    type Item = LogLine;

    fn next(&mut self) -> Option<Self::Item> {
        self.indexer.next()
    }
}

impl<'a, R: IndexedLog> DoubleEndedIterator for SaneLines<'a, R> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.indexer.next_back()
    }
}


#[test]
fn sane_index_iter() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let cursor = CursorLogFile::new(file.to_vec());
    let mut log = Log::from(cursor);
    let it = log.iter_lines();
    assert_eq!(it.count(), 6);
}

#[test]
fn sane_index_iter_rev() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndexer::new(cursor.clone());

    let log = SaneLines::new(&mut index);
    let fwd = log.collect::<Vec<_>>();

    let mut index = SaneIndexer::new(cursor);
    let log = SaneLines::new(&mut index);
    log.indexer.seek(100);
    let rev = log.rev().collect::<Vec<_>>();
    let rev = rev.into_iter().rev().collect::<Vec<_>>();

    assert_eq!(fwd, rev);
}

#[test]
fn sane_index_fwd_rev() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let cursor = CursorLogFile::new(file.to_vec());

    let mut log = Log::from(cursor);
    let mut log = log.iter_lines();
    log.next();
    log.next();
    // FIXME: This fails because we iterate from the same pos.  So rev() turns around from 2 and iterates backwards.
    // Is this worth fixing?

    assert_eq!(log.rev().count(), 4);
}


#[test]
fn sane_index_empty() {
    use crate::files::CursorLogFile;
    let file = b"";
    let cursor = CursorLogFile::new(file.to_vec());
    let mut log = Log::from(cursor);
    let mut log = log.iter_lines();
    assert_eq!(log.next(), None);
}


#[test]
fn sane_index_out_of_range() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let cursor = CursorLogFile::new(file.to_vec());
    let mut log = Log::from(cursor);
    let mut log = log.iter_lines_from(100);
    assert_eq!(log.next(), None);
}


#[test]
fn sane_index_rev_out_of_range() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let cursor = CursorLogFile::new(file.to_vec());
    let mut log = Log::from(cursor);
    let mut log = log.iter_lines_from(100);
    assert_eq!(log.next_back(), None);
}

#[test]
fn sane_index_rev_line_zero() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let cursor = CursorLogFile::new(file.to_vec());
    let mut log = Log::from(cursor);
    let mut log = log.iter_lines_from(5);
    assert!(log.next_back().is_some());
    assert!(log.next_back().is_none());
}
