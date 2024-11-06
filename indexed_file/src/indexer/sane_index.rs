use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::io::BufRead;


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

// struct Range {
//     start: usize,
//     end: usize,
// }

pub struct SaneIndex {
    index: BTreeSet<Waypoint>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Waypoint {
    /// The start of a line; e.g., first line starts at 0
    Mapped(usize),

    /// An uncharted region; beware of index shift. if we find \n at 0, the next line starts at 1.
    /// Range bytes we have to search is in [start, end)
    /// Range of Mapped we may discover is in (start, end]
    Unmapped(Range),
}

impl Clone for Waypoint {
    fn clone(&self) -> Self {
        match self {
            Waypoint::Mapped(offset) => Waypoint::Mapped(*offset),
            Waypoint::Unmapped(range) => Waypoint::Unmapped(range.clone()),
        }
    }
}

/// Region of the index and direction we are searching
/// The range is start-inclusive, end-exclusive
pub enum Search {
    Forward(Range),
    Backward(Range),
}

impl Waypoint {
    pub fn cmp_offset(&self) -> usize {
        match self {
            Waypoint::Mapped(offset) => *offset,
            Waypoint::Unmapped(range) => range.start + 1,
        }
    }

    pub fn contains(&self, offset: usize) -> bool {
        match self {
            Waypoint::Mapped(mapped) => offset == *mapped,
            Waypoint::Unmapped(range) => range.contains(&offset),
        }
    }

    pub fn is_mapped(&self) -> bool {
        matches!(self, Waypoint::Mapped(_))
    }

    fn split_at(&self, offset: usize) -> (Option<Waypoint>, Option<Waypoint>) {
        match self {
            Waypoint::Mapped(_) => unreachable!(),
            Waypoint::Unmapped(range) => {
                let left = if range.start < offset  {
                    Some(Waypoint::Unmapped(range.start..offset.min(range.end)))
                } else {
                    None
                };
                let right = if range.end > offset {
                    Some(Waypoint::Unmapped(offset.max(range.start)..range.end))
                } else {
                    None
                };
                (left, right)
            }
        }
    }
}


impl Ord for Waypoint {
    // unmapped regions are sorted relative to their start offset
    fn cmp(&self, other: &Self) -> Ordering {
        let this = self.cmp_offset().cmp(&other.cmp_offset());
        match this {
            Ordering::Equal => {
                // If the offsets are equal, sort mapped before unmapped
                match self {
                    Waypoint::Mapped(_) => match other {
                        Waypoint::Mapped(_) => Ordering::Equal,
                        _ => Ordering::Less,
                    },
                    Waypoint::Unmapped(range) =>  match other {
                        Waypoint::Unmapped(other_range) => range.end.cmp(&other_range.end),
                        _ => Ordering::Greater,
                    }
                }
            }
            _ => this,
        }
    }
}

impl PartialOrd for Waypoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Default for SaneIndex {
    fn default() -> Self {
        SaneIndex {
            index: BTreeSet::from([Waypoint::Unmapped(0..IMAX)]),
        }
    }
}

impl SaneIndex {
    pub fn new() -> Self {
        Self::default()
    }

    fn find_colliding_gap(&self, range: &Range) -> Option<&Waypoint> {
        let frontier0 = self.index
                .range(Waypoint::Mapped(0)..Waypoint::Mapped(range.end + 1))
                .rev()
                .filter(|waypoint| !waypoint.is_mapped())
                .take_while(|waypoint| waypoint.contains(range.start));
        let frontier1 = self.index
                .range(Waypoint::Mapped(range.start)..Waypoint::Mapped(IMAX))
                .filter(|waypoint| !waypoint.is_mapped())
                .take_while(|waypoint| waypoint.contains(range.end));
        let hits: BTreeSet<&Waypoint> = frontier1.chain(frontier0).collect();
        assert!(hits.len() <= 1);
        if let Some(hit) = hits.last() {
            Some(*hit)
        } else {
            None
        }
    }

    fn resolve_gap(&mut self, gap: Range) {
        // Find the Unmapped region that contains the gap and split it or remove it.
        let mut to_remove : Option<Waypoint> = None;
        let mut to_add = BTreeSet::new();
        if let Some(unmapped) = self.find_colliding_gap(&gap) {
            assert!(!unmapped.is_mapped());
            let (left, middle) = unmapped.split_at(gap.start);
            let (_, right) = middle.unwrap().split_at(gap.end);
            if let Some(left) = left {
                to_add.insert(left);
            }
            if let Some(right) = right {
                to_add.insert(right);
            }
            to_remove = Some(unmapped.clone());
        }
        if let Some(to_remove) = to_remove {
            self.index.remove(&to_remove);
        }
        self.index.extend(to_add);
    }

    pub fn insert(&mut self, offsets: &[usize], range: Range) {
        self.resolve_gap(range.clone());
        for offset in offsets {
            assert!(range.contains(offset) || range.end == *offset);
            self.index.insert(Waypoint::Mapped(*offset));
        }
    }

    pub fn search(&self, search: Search) -> impl Iterator<Item = &Waypoint> {
        match search {
            Search::Forward(range) => {
                self.index.range(Waypoint::Mapped(range.start)..Waypoint::Mapped(range.end))
            }
            Search::Backward(range) => {
                // FIXME: Need to reverse this iterator before using it
                self.index.range(Waypoint::Mapped(range.start)..Waypoint::Mapped(range.end))
            }
        }
    }

    // Parse lines from a BufRead
    pub fn parse_bufread<R: BufRead>(&mut self, source: &mut R, offset: usize, len: usize) -> std::io::Result<usize> {
        /* Alternative:
            let mut pos = offset;
            let newlines = source.lines()
                .map(|x| { pos += x.len() + 1; pos });
            self.line_offsets.extend(newlines);
            */
        let mut pos = offset;
        let end = offset + len;
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
        Ok(pos - offset)
    }

    pub fn parse_chunk(&mut self, offset: usize, chunk: &[u8]) {
        let mut offsets: Vec<usize> = chunk.iter().enumerate()
            .filter(|(_, byte)| **byte == b'\n')
            .map(|(i, _)| offset + i + 1)
            .collect();
        if offset == 0 {
            offsets.push(0);
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
    let mut rando:Vec<usize> = (0..=66).collect::<Vec<_>>();
    rando.shuffle(&mut thread_rng());
    let mut start = 0;
    let mut cuts:Vec<&usize> = rando.iter().take(rando.len()/3).collect();
    cuts.push(&67);
    cuts.sort();
    let cuts = cuts.iter().map(|i| { let s = start; start = **i; s..**i }).collect::<Vec<_>>();
    for i in cuts {
        index.parse_chunk(i.start, file[i].as_bytes());
    }
    assert_eq!(index.index.iter().cloned().collect::<Vec<_>>(), vec![Mapped(0), Mapped(13), Mapped(14), Mapped(30), Mapped(51), Mapped(52), Mapped(67), Unmapped(67..IMAX)]);
}
