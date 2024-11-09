/// SaneLines combines a SaneIndex with a LogFile to provide an iterator over lines in a log file.
/// It's only an index.

use crate::files::LogFile;

use super::{sane_index::SaneIndex, waypoint::Waypoint};

type Range = std::ops::Range<usize>;

struct SaneLines<'a, R: LogFile> {
    index: &'a mut SaneIndex,
    source: &'a mut R,
    range: Range,
}

impl<'a, R: LogFile> SaneLines<'a, R> {
    pub fn new(index: &'a mut SaneIndex, source: &'a mut R, range: Range) -> Self {
        SaneLines {
            index,
            source,
            range,
        }
    }

    fn resolve_gap(&mut self, gap: Range) -> std::io::Result<usize> {
        // Parse part or all of the gap and add it to our mapped index.
        self.source.seek(std::io::SeekFrom::Start(gap.start as u64))?;
        self.index.parse_bufread(self.source, &gap)
    }

    fn read_line(&mut self, offset: usize) -> (usize, Option<(usize, String)>) {
        let mut line = String::new();
        self.source.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
        // FIXME: make this safe for non-utf-8 sequences?
        let bytes = self.source.read_line(&mut line).unwrap();
        let logline = if bytes >  0 {
            Some((offset, line))
        } else {
            None
        };
        (bytes, logline)
    }

    fn next_line(&mut self, offset: usize) -> Option<(usize, String)> {
        let (bytes, line) = self.read_line(offset);
        self.range.start = offset + bytes;
        line
    }

    fn prev_line(&mut self, offset: usize) -> Option<(usize, String)> {
        let (bytes, line) = self.read_line(offset);
        self.range.end = offset;
        line
    }
}

impl<'a, R: LogFile> Iterator for SaneLines<'a, R> {
    type Item = (usize, String);

    fn next(&mut self) -> Option<Self::Item> {
        for _ in 0..5 {
            let mut it = self.index.index.range(Waypoint::Mapped(self.range.start)..Waypoint::Mapped(self.range.end));
            match it.next() {
                Some(Waypoint::Mapped(offset)) => {
                    return self.next_line(*offset)
                },
                None => return None,
                Some(Waypoint::Unmapped(range)) => {
                    if range.start >= self.source.len() {
                        return None;
                    }
                    let start = range.start;
                    let chunk_size = 1024*1024;
                    let end = range.end.max(self.source.len()).min(start + chunk_size);
                    // FIXME: return errors
                    let _ = self.resolve_gap(start..end);
                },
            };
        }
        unreachable!();
    }
}

impl<'a, R: LogFile> DoubleEndedIterator for SaneLines<'a, R> {
    fn next_back(&mut self) -> Option<Self::Item> {
        for _ in 0..5 {
            let mut it = self.index.index.range(Waypoint::Mapped(self.range.start)..Waypoint::Mapped(self.range.end)).rev();
            match it.next() {
                Some(Waypoint::Mapped(offset)) => {
                    return self.prev_line(*offset)
                },
                None => return None,
                Some(Waypoint::Unmapped(range)) => {
                    if range.start >= self.source.len() {
                        self.range.end = self.source.len() - 1;
                        continue;
                    }
                    let start = range.start.min(self.source.len());
                    let end = range.end.min(self.source.len());
                    let chunk_size = 1024*1024;
                    let start = start.max(end.saturating_sub(chunk_size));
                    // FIXME: return errors
                    let _ = self.resolve_gap(start..end);
                },
            }
        }
        unreachable!();
    }
}


#[test]
fn sane_index_iter() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();

    let log = SaneLines::new(&mut index, &mut cursor, 0..100);
    assert_eq!(log.count(), 6);
}

#[test]
fn sane_index_iter_rev() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();

    let log = SaneLines::new(&mut index, &mut cursor, 0..100);
    let fwd = log.collect::<Vec<_>>();

    let mut index = SaneIndex::new();
    let log = SaneLines::new(&mut index, &mut cursor, 0..100);
    let rev = log.rev().collect::<Vec<_>>();
    let rev = rev.into_iter().rev().collect::<Vec<_>>();

    assert_eq!(fwd, rev);
}

#[test]
fn sane_index_fwd_rev() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();

    let mut log = SaneLines::new(&mut index, &mut cursor, 0..100);
    log.next();
    log.next();

    assert_eq!(log.rev().count(), 4);
}


#[test]
fn sane_index_empty() {
    use crate::files::CursorLogFile;
    let file = b"";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();
    let mut log = SaneLines::new(&mut index, &mut cursor, 0..100);
    assert_eq!(log.next(), None);
}


#[test]
fn sane_index_out_of_range() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();
    let mut log = SaneLines::new(&mut index, &mut cursor, 100..200);
    assert_eq!(log.next(), None);
}


#[test]
fn sane_index_rev_out_of_range() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();
    let mut log = SaneLines::new(&mut index, &mut cursor, 100..200);
    assert_eq!(log.next_back(), None);
}

#[test]
fn sane_index_rev_line_zero() {
    use crate::files::CursorLogFile;
    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = CursorLogFile::new(file.to_vec());
    let mut index = SaneIndex::new();
    let mut log = SaneLines::new(&mut index, &mut cursor, 0..5);
    assert!(log.next_back().is_some());
    assert!(log.next_back().is_none());
}
