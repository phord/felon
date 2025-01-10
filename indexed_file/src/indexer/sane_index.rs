use std::collections::VecDeque;

use super::{indexed_log::IndexStats, waypoint::{Position, VirtualPosition, Waypoint}};


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
/// We read the first line and map it:  [ Mapped(0..13), Mapped(13..14), Unmapped(13..IMAX) ]
///
/// -> When we read the last line, we leave an umapped region at the end in case the file grows later.
/// We read the last line and map it:   [ Mapped(0..13), Mapped(13..14), Unmapped(13..51), Mapped(52), Unmapped(67..IMAX)]
/// We read the second line and map it: [ Mapped(0..13), Mapped(13..14), Mapped(14..30), Unmapped(14..51), Mapped(52), Unmapped(67..IMAX) ]
/// Finally we scan the middle region:  [ Mapped(0..13), Mapped(14..30), Mapped(30..51), Mapped(51..52), Mapped(52), Unmapped(67..IMAX) ]
///
/// Suppose we mapped the middle section of the file first.
/// Initially the file is unmapped:     [ Unmapped(0..IMAX) ]
/// We scan bytes 10 to 39:             [ Unmapped(0..10), Mapped(13..14), Mapped(14..30), Mapped(30..39), Unmapped(40..IMAX) ]
///
/// Note we always assume there is a line at Mapped(0..13).  But it may not be inserted in every index.

/// Updated to use a splitvec-style implementation when growing in the middle.
/// Each internal vector either has a single Unmapped(range) or more Mapped(offset) values.


const IMAX:usize = usize::MAX;
type Range = std::ops::Range<usize>;

type IndexVec = Vec<VecDeque<Waypoint>>;
pub type IndexIndex = (usize, usize);

pub struct SaneIndex {
    pub(crate) index: IndexVec,
    pub(crate) stats: IndexStats,
}

impl Default for SaneIndex {
    fn default() -> Self {
        SaneIndex {
            index: vec![VecDeque::from([Waypoint::Unmapped(0..IMAX)])],
            stats: IndexStats::default(),
        }
    }
}

impl SaneIndex {
    pub fn new(name: String) -> Self {
        SaneIndex {
            stats: IndexStats::new(name),
            ..SaneIndex::default()
        }
    }

    pub fn index_prev(&self, idx: IndexIndex) -> Option<IndexIndex> {
        let (i, j) = idx;
        if j > 0 {
            Some((i, j - 1))
        } else if i > 0 {
            Some((i - 1, self.index[i - 1].len() - 1))
        } else {
            None
        }
    }

    pub fn index_next(&self, idx: IndexIndex) -> Option<IndexIndex> {
        let (i, j) = idx;
        if j + 1 < self.index[i].len() {
            Some((i, j + 1))
        } else if i + 1 < self.index.len() {
            Some((i + 1, 0))
        } else {
            None
        }
    }

    pub fn index_valid(&self, idx: IndexIndex) -> bool {
        let (i, j) = idx;
        i < self.index.len() && j < self.index[i].len()
    }

    pub fn value(&self, idx: IndexIndex) -> &Waypoint {
        let (i, j) = idx;
        &self.index[i][j]
    }

    pub(crate) fn seek_gap(&self, pos: &Position) -> Position {
        let pos = pos.resolve(self);

        if let Position::Existing((row, _), _) = pos {
            for (i, row) in self.index[row..].iter().enumerate() {
                if !row.front().unwrap().is_mapped() {
                    return Position::Existing((i, 0), row[0].clone());
                }
            }
        }

        // Didn't find any more gaps
        Position::Virtual(VirtualPosition::End)
    }

    /// Find the index holding the given offset, or where it would be inserted if none found.
    pub(crate) fn search(&self, offset: usize) -> IndexIndex {
        if self.index.is_empty() {
            // This returns a pointer past the end when index.is_empty().  Is it bad?
            return (0, 0);
        }
        let find = self.index.binary_search_by(|v| {v[0].cmp_offset().cmp(&offset)});
        let ndx  = match find {
            // Found the matching index in the first element of the row.  What luck!
            Ok(i) => (i, 0),
            // Found the row where the index should exist (at i-1)
            Err(i) => {
                let i = i.saturating_sub(1);
                match self.index[i].binary_search_by(|v| v.cmp_offset().cmp(&offset)) {
                    Ok(j) => (i, j),
                    Err(j) => {
                        if j == self.index[i].len() {
                            if i + 1 < self.index.len() {
                                (i + 1, 0)
                            } else {
                                // Never return pointer past end.  is this right???  It breaks a couple of literal tests...
                                (i, j-1)
                            }
                        } else {
                            (i, j)
                        }
                    },
                }
            },
        };

        if let Some(prev) = self.index_prev(ndx) {
            if self.value(prev).contains(offset) {
                return prev;
            }
        }
        if self.index_valid(ndx) && offset > self.value(ndx).cmp_offset() {
            if let Some(next) = self.index_next(ndx) {
                return next;
            }
        }
        ndx
    }

    pub(crate) fn next(&self, pos: &Position) -> Position {
        pos.next(self)
    }

    pub(crate) fn next_back(&self, pos: &Position) -> Position {
        pos.next_back(self)
    }

    // Resolves gap which must be in a Position::Existing(Mapped)
    // Splits the gap if it's in the middle of the range and returns the index of the 2nd gap;
    // otherwise, shrinks the gap and returns the index of the adjacent row where a new waypoint can be appended.
    fn resolve_gap_at(&mut self, pos: &Position, gap: &Range) -> usize {
        let (ndx, unmapped) = match pos {
            Position::Existing(ndx, waypoint) => (ndx, waypoint),
            _ => panic!("Can only resolve gaps at unmapped positions"),
        };

        assert!(!unmapped.is_mapped());
        assert!(unmapped.end_offset() >= gap.end);
        assert!(unmapped.cmp_offset() <= gap.start);

        let (i, j) = ndx;
        assert!(*j == 0, "unmapped regions should be in their own vector");
        assert!(self.index[*i].len() == 1, "unmapped regions should be in their own vector");

        // Given a gap in the index, split it across 1 to 3 ranges compared to our range.
        // Left and/or Right may be None if the gap and range ends align.
        //   [--------- Actual Gap ---------]
        //          [ --- Range --- ]
        //   [ Left |     Middle    | Right ]
        //             ^^removed^^
        // Four possibilities:
        //    Removed part  Action      Returns        Val points to   To insert cells
        // 1. Left edge     shrink gap  row            Remainder       Append to row - 1
        // 2. Right edge    shrink gap  row            Remainder       Prepend to row + 1
        // 3. Middle        split gap   row+1          Right half      Insert row
        // 4. Whole gap     remove gap  row            empty row       Use empty row

        let (left, middle) = unmapped.split_at(gap.start);
        let right =
            if let Some(middle) = middle {
                let (_, right) = middle.split_at(gap.end);
                right
            } else {
                unreachable!("We should always find a middle unless our gaps don't overlap");
            };
        if left.is_none() && right.is_none() {
            // This gap is completely removed.  Clear the row and return the empty row.
            // Note: We could remove the row and let the caller insert into the next row instead.
            self.index[*i].clear();
            *i
        } else if left.is_some() && right.is_some() {
            // Split into two gaps on either side of our range.
            // 1. Insert new row with left-gap before our position
            self.index.insert(*i, VecDeque::from([left.unwrap()]));
            // 2. Replace original gap with our right-gap
            *self.index[i + 1].get_mut(0).unwrap() = right.unwrap();
            // Next line should go before the 2nd half
            i + 1
        } else if let Some(left) = left {
            // Replace the old gap with just the left part
            *self.index[*i].get_mut(0).unwrap() = left;
            // The new line should go in the next slot
            *i
        } else {
            assert!(right.is_some());
            // Replace the old gap with just the right part
            *self.index[*i].get_mut(0).unwrap() = right.unwrap();
            // The new line should go in the previous slot
            *i
        }
    }

    /// Find the Unmapped region that contains the given gap;
    /// Returns a Position
    #[cfg(test)]
    fn find_gap(&mut self, gap: &Range) -> Position {
        let mut ndx = self.search(gap.start);
        if self.value(ndx).is_mapped() {
            if let Some(next) = self.index_next(ndx) {
                ndx = next;
            }
        } else if let Some(prev) = self.index_prev(ndx) {
            if self.value(prev).contains(gap.start) {
                ndx = prev;
            }
        }
        assert!(self.index_valid(ndx));

        Position::Existing(ndx, self.value(ndx).clone())
    }

    // Insert one new waypoint covering given range. New waypoint must be contained in a gap.
    // Returns position of new waypoint
    // This is only used in tests. Prod code calls insert_one directly
    #[cfg(test)]
    pub fn insert(&mut self, range: &Range) -> Position {
        let pos = self.find_gap(range);
        self.insert_one(&pos, range)
    }

    // Erase a gap covering a given range.
    // Prefer to call erase_gap directly
    // Returns position of next waypoint
    // This is only used in tests. Prod code calls erase_gap directly
    #[cfg(test)]
    pub fn erase(&mut self, range: &Range) -> Position {
        let pos = self.find_gap(range);
        self.erase_gap(&pos, range)
    }

    // Clear the gap at the given Position. Returns the a guide indicating where replacements should be inserted, if desired
    fn clear_gap(&mut self, pos: &Position, range: &Range) -> usize {
        let pos = pos.resolve(self);
        // Find exact gap that covers the region. We will shrink it or replace it.
        let gap_range = pos.region();
        let gap_range = gap_range.start.max(range.start)..gap_range.end.min(range.end);
        assert!(!gap_range.is_empty());
        if !gap_range.is_empty() {
            self.stats.bytes_indexed += gap_range.end - gap_range.start;
        }
        self.resolve_gap_at(&pos, &gap_range)
    }

    // Remove the gap at the given Position. No lines are to be added.  Returns ptr to remaining gap, or row after removed gap.
    pub fn erase_gap(&mut self, pos: &Position, range: &Range) -> Position {
        let row = self.clear_gap(pos, range);
        if self.index[row].is_empty() {
            self.index.remove(row);

        }
        if row == self.index.len() {
            Position::Virtual(VirtualPosition::End)
        } else {
            Position::Existing((row, 0), self.index[row][0].clone())
        }
    }

    // Insert a new waypoint at the given position (in a Unmapped range).  Returns the Position of the new waypoint
    pub fn insert_one(&mut self, pos: &Position, range: &Range) -> Position {
        let row = self.clear_gap(pos, range);
        self.stats.lines_indexed += 1;

        // Returned slot is remainder of gap, if any.  We need to insert before or after that gap.
        // Find row on other side of gap make sure we can insert there.  If it's unmapped, we need to add a row.
        let row =
            if let Some(first) = self.index[row].front() {
                assert!(!first.is_mapped(), "Expect pointer to remainder of gap");
                let other = if range.start < first.cmp_offset() {
                    // Inserting to the left of this gap (end of previous row)
                    row.saturating_sub(1)
                } else {
                    // Insert to the right of this gap (beginning of next row)
                    assert!(range.start >= first.end_offset());
                    row + 1
                };
                if row == other                             // row == other == 0: need to to insert a row
                    || !self.index[other][0].is_mapped()    // target row is unmapped; insert new row to hold mapped
                {
                    // We have to insert an empty row
                    self.index.insert(row.max(other), VecDeque::new());
                    row.max(other)
                } else {
                    other
                }
            } else {
                row
            };

        // The waypoint to insert
        let waypoint = Waypoint::Mapped(range.start..range.end);
        let waypoint_pos = waypoint.clone();

        // Now we are either appending or prepending to the list
        let col =
            if let Some(first) = self.index[row].front() {
                if first < &waypoint {
                    // Must be appending
                    assert!(self.index[row].back().unwrap() < &waypoint);
                    self.index[row].push_back(waypoint);
                    self.index[row].len() - 1
                } else {
                    self.index[row].insert(0, waypoint);
                    0
                }
            } else {
                self.index[row].push_back(waypoint);
                0
            };
        Position::Existing((row, col), waypoint_pos)
    }

    #[cfg(test)]
    pub(crate) fn iter(&self) -> SaneIter {
        SaneIter::new(self)
    }
}

pub struct SaneIter<'a> {
    index: &'a SaneIndex,
    pos: Position,
}

#[cfg(test)]
impl<'a> SaneIter<'a> {
    fn new(index: &'a SaneIndex) -> Self {
        SaneIter {
            pos: Position::Virtual(VirtualPosition::Start).resolve(index),
            index,
        }
    }
}

impl<'a> Iterator for SaneIter<'a> {
    type Item = Waypoint;

    fn next(&mut self) -> Option<Self::Item> {
        let p = self.pos.clone();
        self.pos = self.pos.next(self.index);
        match p {
            Position::Existing(_, waypoint) => {
                Some(waypoint)
            },
            _ => {
                self.pos = Position::Virtual(VirtualPosition::Invalid);
                None
            },
        }
    }
}


#[test]
fn sane_index_basic() {
    let mut index = SaneIndex::default();
    index.insert(&(0..13));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(0..13), Waypoint::Unmapped(13..IMAX)]);
    index.insert(&(13..14));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(0..13), Waypoint::Mapped(13..14), Waypoint::Unmapped(14..IMAX)]);
    index.insert(&(14..30));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(0..13), Waypoint::Mapped(13..14), Waypoint::Mapped(14..30), Waypoint::Unmapped(30..IMAX)]);
    index.insert(&(30..51));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(0..13), Waypoint::Mapped(13..14), Waypoint::Mapped(14..30), Waypoint::Mapped(30..51), Waypoint::Unmapped(51..IMAX)]);
    index.insert(&(51..52));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(0..13), Waypoint::Mapped(13..14), Waypoint::Mapped(14..30), Waypoint::Mapped(30..51), Waypoint::Mapped(51..52), Waypoint::Unmapped(52..IMAX)]);
    index.erase(&(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(0..13), Waypoint::Mapped(13..14), Waypoint::Mapped(14..30), Waypoint::Mapped(30..51), Waypoint::Mapped(51..52), Waypoint::Unmapped(67..IMAX)]);
    assert_eq!(index.index.len(), 2);
}

#[test]
fn sane_index_basic_rev() {
    let mut index = SaneIndex::default();
    index.erase(&(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Unmapped(0..52), Waypoint::Unmapped(67..IMAX)]);
    index.insert(&(13..14));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Unmapped(0..13), Waypoint::Mapped(13..14), Waypoint::Unmapped(14..52), Waypoint::Unmapped(67..IMAX)]);
    index.erase(&(0..13));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(13..14), Waypoint::Unmapped(14..52), Waypoint::Unmapped(67..IMAX)]);
    index.insert(&(14..30));
    assert_eq!(index.iter().collect::<Vec<_>>(), vec![Waypoint::Mapped(13..14), Waypoint::Mapped(14..30), Waypoint::Unmapped(30..52), Waypoint::Unmapped(67..IMAX)]);
}
