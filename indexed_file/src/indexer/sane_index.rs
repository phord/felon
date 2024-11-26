use std::io::BufRead;
use crate::indexer::waypoint;

use super::waypoint::{Position, VirtualPosition, Waypoint};


/// SaneIndex
/// Holds a map of the explored regions of the file.
///      0: Hello, world\n
///     13: \n
///     14: This is a test.\n
///     30: This is only a test.\n
///     51: \n
///     52: End of message\n
///     67:
///
/// This file has 67 bytes.
/// Initially the file is unmapped:     [ Unmapped(0..IMAX) ]
///
/// -> When we read the first line, we learn the offset of the second one. Notice unmapped still includes the start of the 2nd line.
/// We read the first line and map it:  [ Mapped(0), Mapped(13), Unmapped(13..IMAX) ]
///
/// -> When we read the last line, we leave an umapped region at the end in case the file grows later.
/// We read the last line and map it:   [ Mapped(0), Mapped(13), Unmapped(13..51), Mapped(52), Unmapped(67..IMAX)]
/// We read the second line and map it: [ Mapped(0), Mapped(13), Mapped(14), Unmapped(14..51), Mapped(52), Unmapped(67..IMAX) ]
/// Finally we scan the middle region:  [ Mapped(0), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Unmapped(67..IMAX) ]
///
/// Suppose we mapped the middle section of the file first.
/// Initially the file is unmapped:     [ Unmapped(0..IMAX) ]
/// We scan bytes 10 to 39:             [ Unmapped(0..10), Mapped(13), Mapped(14), Mapped(30), Unmapped(40..IMAX) ]
///
/// Note we always assume there is a line at Mapped(0).  But it may not be inserted in every index.

const IMAX:usize = usize::MAX;
type Range = std::ops::Range<usize>;


pub struct SaneIndex {
    pub(crate) index: Vec<Waypoint>,
}

impl Default for SaneIndex {
    fn default() -> Self {
        SaneIndex {
            index: vec![Waypoint::Unmapped(0..IMAX)],
        }
    }
}

impl SaneIndex {
    pub fn new() -> Self {
        Self::default()
    }

    /// Find the index holding the given offset, or where it would be inserted if none found.
    pub(crate) fn search(&self, offset: usize) -> usize {
        let find = self.index.binary_search(&Waypoint::Mapped(offset));
        let i = match find {
            // Found the matching index
            Ok(i) => i,
            // Found where the index should be inserted
            Err(i) => i,
        };
        if i > 0 && self.index[i - 1].contains(offset) {
            i - 1
        } else if i < self.index.len() && offset > self.index[i].cmp_offset() {
            i + 1
        } else {
            i
        }
    }

    pub(crate) fn next(&self, pos: Position) -> Position {
        let mut pos = pos;
        pos.next(&self);
        pos
    }

    pub(crate) fn next_back(&self, pos: Position) -> Position {
        let mut pos = pos;
        pos.next_back(&self);
        pos
    }

    fn resolve_gap(&mut self, gap: Range) {
        // Find the Unmapped region that contains the gap and split it or remove it.
        let mut to_add = Vec::new();
        let mut i = self.search(gap.start);
        if i + 1 < self.index.len() && self.index[i].is_mapped() {
            i += 1;
        } else if i > 0 && self.index[i - 1].contains(gap.start) {
            i -= 1;
        }
        assert!(i < self.index.len());

        let unmapped = &self.index[i];
        assert!(!unmapped.is_mapped());
        assert!(unmapped.end_offset() >= gap.end);
        assert!(unmapped.cmp_offset() <= gap.start);

        let (left, middle) = unmapped.split_at(gap.start);
        let (_, right) = middle.unwrap().split_at(gap.end);
        if let Some(left) = left {
            to_add.push(left);
        }
        if let Some(right) = right {
            to_add.push(right);
        }

        // We have to add 0, 1 or 2 things and we have to remove 1.
        if to_add.len() == 0 {
            // Nothing to insert; remove only
            self.index.remove(i);
        } else {
            self.index[i] = to_add.pop().unwrap();
            for waypoint in to_add.into_iter().rev() {
                self.index.insert(i, waypoint);
            }
        }
    }

    pub fn insert(&mut self, offsets: &[usize], range: Range) {
        // Remove gaps that covered the region
        self.resolve_gap(range.clone());

        if let Some(val) = offsets.last() {
            // Insert the new offsets into the index
            // dbg!(val);
            let i = self.search(*val);
            // dbg!(i);
            // FIXME: Insert the whole slice at once; what kind of container can do that?
            for offset in offsets.into_iter().rev() {
                assert!(range.contains(&offset) || range.end == *offset);
                self.index.insert(i, Waypoint::Mapped(*offset));
            }
        }
    }

    // Parse lines from a BufRead
    pub fn parse_bufread<R: BufRead>(&mut self, source: &mut R, range: &Range) -> std::io::Result<usize> {
        /* We want to do this, except it takes ownership of the source:
            let mut pos = offset;
            let newlines = source.lines()
                .map(|x| { pos += x.len() + 1; pos });
            self.line_offsets.extend(newlines);
            */
        let mut pos = range.start;
        let end = range.end;
        while pos < end {
            let bytes =
                match source.fill_buf() {
                    Ok(buf) => {
                        if buf.is_empty() {
                            break       // EOF
                        }
                        let len = buf.len().min(end - pos);
                        self.parse_chunk(pos, &buf[..len]);
                        len
                    },
                    Err(e) => {
                        return std::io::Result::Err(e)
                    },
                };
            pos += bytes;
            source.consume(bytes);
        }
        Ok(pos - range.start)
    }

    pub fn parse_chunk(&mut self, offset: usize, chunk: &[u8]) {
        let mut offsets: Vec<usize> = chunk.iter().enumerate()
            .filter(|(_, byte)| **byte == b'\n')
            .map(|(i, _)| offset + i + 1)
            .collect();
        if offset == 0 {
            offsets.insert(0, 0);
        }
        self.insert(&offsets, offset..offset + chunk.len());
    }
}


#[test]
fn sane_index_basic() {
    let mut index = SaneIndex::new();
    index.insert(&[0], 0..13);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(0), Waypoint::Unmapped(13..IMAX)]);
    index.insert(&[13], 13..14);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(0), Waypoint::Mapped(13), Waypoint::Unmapped(14..IMAX)]);
    index.insert(&[14], 14..30);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(0), Waypoint::Mapped(13), Waypoint::Mapped(14), Waypoint::Unmapped(30..IMAX)]);
    index.insert(&[30], 30..51);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(0), Waypoint::Mapped(13), Waypoint::Mapped(14), Waypoint::Mapped(30), Waypoint::Unmapped(51..IMAX)]);
    index.insert(&[51], 51..52);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(0), Waypoint::Mapped(13), Waypoint::Mapped(14), Waypoint::Mapped(30), Waypoint::Mapped(51), Waypoint::Unmapped(52..IMAX)]);
    index.insert(&[], 52..67);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(0), Waypoint::Mapped(13), Waypoint::Mapped(14), Waypoint::Mapped(30), Waypoint::Mapped(51), Waypoint::Unmapped(67..IMAX)]);
    assert_eq!(index.index.len(), 6);
}

#[test]
fn sane_index_basic_rev() {
    let mut index = SaneIndex::new();
    index.insert(&[], 52..67);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Unmapped(0..52), Waypoint::Unmapped(67..IMAX)]);
    index.insert(&[13], 13..14);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Unmapped(0..13), Waypoint::Mapped(13), Waypoint::Unmapped(14..52), Waypoint::Unmapped(67..IMAX)]);
    index.insert(&[], 0..13);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(13), Waypoint::Unmapped(14..52), Waypoint::Unmapped(67..IMAX)]);
    index.insert(&[14], 14..30);
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Waypoint::Mapped(13), Waypoint::Mapped(14), Waypoint::Unmapped(30..52), Waypoint::Unmapped(67..IMAX)]);
}


#[test]
fn sane_index_parse_basic() {
    use Waypoint::*;
    let mut index = SaneIndex::new();
    let file = "Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    index.parse_chunk(0, file.as_bytes());
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Mapped(0), Mapped(13), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
}

#[test]
fn sane_index_parse_chunks() {
    use Waypoint::*;
    let mut index = SaneIndex::new();
    let file = "Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let start = 35;
    index.parse_chunk(start, file[start..].as_bytes());
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Unmapped(0..start), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
    index.parse_chunk(0, file[..start].as_bytes());
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Mapped(0), Mapped(13), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
}

#[test]
fn sane_index_parse_chunks_random_bytes() {
    use Waypoint::*;
    use rand::thread_rng;
    use rand::seq::SliceRandom;

    let mut index = SaneIndex::new();
    let file = "Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut rando:Vec<usize> = (0..=66).collect::<Vec<_>>();
    rando.shuffle(&mut thread_rng());
    for i in rando {
        index.parse_chunk(i, file[i..i+1].as_bytes());
    }
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Mapped(0), Mapped(13), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
}


#[test]
fn sane_index_parse_chunks_random_chunks() {
    use Waypoint::*;
    use rand::thread_rng;
    use rand::seq::SliceRandom;

    let mut index = SaneIndex::new();
    let file = "Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut rando:Vec<usize> = (1..=66).collect::<Vec<_>>();
    rando.shuffle(&mut thread_rng());
    let mut start = 0;

    // Collect 1/3 of the byte offsets from the file.
    let mut cuts:Vec<&usize> = rando.iter().take(rando.len()/3).collect();

    // Always ensure that the last byte is included.
    cuts.push(&67);
    cuts.sort();
    let mut cuts = cuts.iter().map(|i| { let s = start; start = **i; s..**i }).collect::<Vec<_>>();

    // Resolve the ranges in random order
    cuts.shuffle(&mut thread_rng());
    for i in cuts {
        index.parse_chunk(i.start, file[i].as_bytes());
    }
    assert_eq!(index.index.to_vec(), vec![Mapped(0), Mapped(13), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
}

#[test]
fn sane_index_full_bufread() {
    use Waypoint::*;

    let file = b"Hello, world\n\nThis is a test.\nThis is only a test.\n\nEnd of message\n";
    let mut cursor = std::io::Cursor::new(file);

    let mut index = SaneIndex::new();

    index.parse_bufread(&mut cursor, &(0..100)).unwrap();
    assert_eq!(index.index.to_vec(), vec![Mapped(0), Mapped(13), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
}
