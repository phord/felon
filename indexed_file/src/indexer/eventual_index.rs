use std::cmp::Ordering;

use crate::indexer::index::Index;

// An index of some lines in a file, possibly with gaps, but eventually a whole index
pub struct EventualIndex {
    indexes: Vec<Index>,
}

// A cursor, representing a location in the EventualIndex
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Location {
    // A conceptual location, like "Start of file". Use resolve() to get a real location.
    Virtual(VirtualLocation),

    // A location we have indexed before; we know where it is.
    Indexed(IndexRef),

    // A location we have not indexed yet; we need to get more information.
    Gap(GapRange),

    // This location will never exist
    Invalid,
}

impl Location {
    /// Find the offset of the located line, or None if no line located yet
    pub fn found_offset(&self) -> Option<usize> {
        match self {
            Location::Indexed(r) => Some(r.offset),
            _ => None,
        }
    }

    pub fn reached(&self) -> bool {
        match self {
            Location::Indexed(r) => r.next.reached(r.offset),
            _ => false,
        }
    }


    #[inline]
    pub fn is_gap(&self) -> bool {
        matches!(self, Location::Gap(_))
    }

    #[inline]
    pub fn is_virtual(&self) -> bool {
        matches!(self, Location::Virtual(_))
    }

    #[inline]
    pub fn is_indexed(&self) -> bool {
        matches!(self, Location::Indexed(_))
    }

    #[inline]
    pub fn is_invalid(&self) -> bool {
        matches!(self, Location::Invalid)
    }

    // Get offset in file of next byte we will scan
    pub fn offset(&self) -> usize {
        use Location::*;
        use VirtualLocation::*;
        match self {
            Virtual(Start) => 0,
            Virtual(End) => usize::MAX,
            Virtual(Before(off)) => off.saturating_sub(1),
            Virtual(AtOrAfter(off)) => *off,
            Indexed(iref) => iref.offset,
            Gap(GapRange{target: off, ..}) => off.value(),

            Invalid => panic!("No offset available for invalid location"),
        }
    }

    // make a portable location we can use with another EventualIndex
    pub fn gap_to_target(self) -> VirtualLocation {
        use Location::*;
        use VirtualLocation::*;
        assert!(self.is_gap());
        match self {
            Gap(GapRange{target, gap, ..}) => {
                let (start, end) = match gap {
                    Missing::Bounded(start, end) => (start, end),
                    Missing::Unbounded(start) => (start, usize::MAX - 1),
                };
                match target {
                    // return a target from the start or the end of the gap, as needed.
                    TargetOffset::AtOrBefore(off) => Before(end.min(off + 1)),
                    TargetOffset::AtOrAfter(off) => AtOrAfter(start.max(off)),
                }
            },
            _ => panic!("Not a gap"),
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset().cmp(&other.offset())
    }
}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Delineates [start, end) of a region of the file.  end is not inclusive.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Missing {
    // Range has start and end; end is not inclusive
    Bounded(usize, usize),

    // Range has start; end is unknown
    Unbounded(usize),
}

// Literally a reference by subscript to the Index/Line in an EventualIndex.
// Invalid if the EventualIndex changes, such as when a prior gap is filled in. But we should be holding a GapRange instead, then.
#[derive(Debug, Copy, Clone)]
pub struct IndexRef {
    pub index: usize,
    pub line: usize,
    pub offset: usize,
    pub next: TargetOffset,
}

impl PartialEq for IndexRef {
    fn eq(&self, other: &Self) -> bool {
        self.offset == other.offset
    }
}
impl Eq for IndexRef {}

// A logical location in a file, like "Start"
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum VirtualLocation {
    Start,
    End,
    Before(usize),      // actually Before, not AtOrBefore.  Why is it different?  :concerned:
    AtOrAfter(usize),
}

impl VirtualLocation {
    pub fn is_before(&self) -> bool {
        matches!(self, VirtualLocation::Before(_))
    }

    pub fn is_after(&self) -> bool {
        matches!(self, VirtualLocation::AtOrAfter(_))
    }

    pub fn offset(&self) -> usize {
        match self {
            VirtualLocation::Before(x) => *x,
            VirtualLocation::AtOrAfter(x) => *x,
            VirtualLocation::Start => 0,
            VirtualLocation::End => usize::MAX,
        }
    }
}

// The target offset we wanted to reach when filling a gap
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TargetOffset {
    AtOrBefore(usize),
    AtOrAfter(usize),
}

impl TargetOffset {
    pub fn value(&self) -> usize {
        match self {
            TargetOffset::AtOrAfter(x) => *x,
            TargetOffset::AtOrBefore(x) => *x,
        }
    }

    pub fn is_after(&self) -> bool {
        match self {
            TargetOffset::AtOrAfter(_) => true,
            TargetOffset::AtOrBefore(_) => false,
        }
    }

    pub fn next_exclusive(&self, offset: usize) -> TargetOffset {
        match self {
            TargetOffset::AtOrAfter(_) => TargetOffset::AtOrAfter(offset + 1),
            TargetOffset::AtOrBefore(_) => TargetOffset::AtOrBefore(offset.saturating_sub(1)),
        }
    }

    pub fn reached(&self, offset: usize) -> bool {
        match self {
            TargetOffset::AtOrAfter(x) => offset >= *x,
            TargetOffset::AtOrBefore(x) => offset <= *x,
        }
    }
}

// A cursor to some gap in the indexed coverage
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
// Position at `target` is not indexed; need to index region from `gap`
pub struct GapRange {
    /// The approximate offset we wanted to reach
    pub target: TargetOffset,

    /// The index after our gap, or indexes.len() if none after
    pub index: usize,

    /// The type and size of the gap
    pub gap: Missing,
}

// Manage the internal representation
impl EventualIndex {
    pub fn new() -> EventualIndex {
        EventualIndex {
            indexes: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: Index) {
        // merge lazily
        self.indexes.push(other);
    }

    /// Insert an explored range into the eventualIndex and optionally add a found line offset.
    /// Location must be a gap.
    /// Returns the indexed Location where the entry was stored, or a gap if no entry provided
    pub fn insert(&mut self, pos: &Location, range: &std::ops::Range<usize>, offset: Option<usize>) -> Location {

        let gap_range = match pos {
            Location::Gap(gap_range) => gap_range,
            _ => panic!("Location not a gap"),
        };
        let ix = gap_range.index;

        let index = if ix > 0 && (self.indexes[ix - 1].touches(&range.start) ||
                                        self.indexes[ix - 1].touches(&range.end)) {
            // Append to previous index if it's adjacent (this is the most efficient option)
            ix - 1
        } else if ix < self.indexes.len() &&
            (self.indexes[ix].touches(&range.start) || self.indexes[ix].touches(&range.end)) {
            // Prepend to next index if it's adjacent
            ix
        } else {
            // No adjacent index exists.  Insert a new zero-sized one. This allows it to be adjacent to our added range.
            self.indexes.insert(ix, Index::new());
            self.indexes[ix].start = range.start;   // Empty range; insert will expand it
            self.indexes[ix].end = range.start;
            ix
        };

        let line = self.indexes[index].insert(range, offset);

        if index > 0 {
            assert!(self.indexes[index - 1].end <= self.indexes[index].start, "Expected indexes to be in order");
        }
        if let Some(offset) = offset {
            Location::Indexed(IndexRef{ index, line, offset, next: gap_range.target })
        } else {
            // Close the gap:

            // TODO: Can we finesse this better to be more efficient?
            // Location::Gap(GapRange{ target: gap_range.target, index, gap: Missing::Bounded(range.start, range.end) })
            if let Some(gap) = self.try_gap_at(index+1, gap_range.target) {
                gap
            } else {
                self.get_location(index, line, gap_range.target)
            }
        }
    }


    pub fn finalize(&mut self) {
        if self.indexes.len() < 2 {
            return;
        }

        self.indexes.sort_by_key(|index| index.start);

        let mut prev = 0;
        for index in self.indexes.iter() {
            assert!(index.start >= prev);
            prev = index.end;
        }

        // Use a fold iterator to concatenate adjacent indexes
        self.indexes = self.indexes
            .drain(..)
            .filter(|index| !index.is_empty())
            .fold(Vec::new(), |mut acc, index| {
                if let Some(last) = acc.last_mut() {
                    if last.adjacent(&index.range()) {
                        last.merge(&index);
                    } else {
                        acc.push(index);
                    }
                } else {
                    acc.push(index);
                }
                acc
            });
    }

    // #[cfg(test)]
    pub fn bytes(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.bytes())
    }

    pub fn lines(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.lines())
    }

    // Return the first indexed byte
    pub fn start(&self) -> usize {
        if let Some(start) = self.indexes.first() {
            start.start
        } else {
            0
        }
    }

    // Return the last indexed byte
    pub fn end(&self) -> usize {
        if let Some(end) = self.indexes.last() {
            end.end
        } else {
            0
        }
    }
}

// Gap handlers
impl EventualIndex {
    // Identify the gap before a given index position and return a Missing() hint to include it.
    // panics if there is no gap
    fn gap_at(&self, pos: usize, target: TargetOffset) -> Location {
        self.try_gap_at(pos, target).unwrap()
    }

    // Describe the gap before the index which includes the target offset
    // If index is not indexed yet, find the gap at the end of indexes
    // Returns None if there is no gap
    fn try_gap_at(&self, index: usize, target: TargetOffset) -> Option<Location> {
        use Missing::{Bounded, Unbounded};

        assert!(index <= self.indexes.len());

        if self.indexes.is_empty() {
            Some(Location::Gap(GapRange { target, index: 0, gap: Unbounded(0) } ))
        } else if index == 0 {
            // gap is at start of file
            let next = self.indexes[index].start;
            if next > 0 {
                Some(Location::Gap(GapRange { target, index: 0, gap: Bounded(0, next) } ))
            } else if self.indexes[0].is_empty() {
                // There is no gap, but also no lines in the file
                Some(Location::Invalid)
            } else {
                // There is no gap at start of file
                None
            }
        } else {
            // gap is after indexes[index-1]
            let prev_index = &self.indexes[index-1];
            let prev = prev_index.end;
            if prev_index.indexes(&target.value()) {
                // Next target is already indexed (line before or after it is already available)
                None
            } else if index == self.indexes.len() {
                // gap is at end of file; return unbounded range
                Some(Location::Gap(GapRange { target, index,  gap: Unbounded(prev) } ))
            } else {
                // Find the gap between two indexes; bracket result by their [end, start)
                let next = self.indexes[index].start;
                if next > prev {
                    Some(Location::Gap(GapRange { target, index, gap: Bounded(prev, next) } ))
                } else {
                    // There is no gap between these indexes
                    assert!(next == prev);
                    None
                }
            }
        }
    }
}

// Cursor functions for EventualIndex
impl EventualIndex {
    // Find index to line that contains a given offset or the gap that needs to be loaded to have it. Somewhat expensive.
    pub fn locate(&self, target: TargetOffset) -> Location {
        // TODO: Trace this fallback finder and ensure it's not being overused.

        let offset = target.value();
        let pos = match self.indexes.binary_search_by(|i| i.contains_offset(&offset)) {
            Ok(found) => {
                let i = &self.indexes[found];
                let line = i.find(offset).unwrap();
                // The found offset may be just before or just after where we want to be.  TargetOffset knows the difference.
                // Fine tune the Location to get the actual line we wanted.
                self.find_location(found, line, target)
            },
            Err(after) => {
                // No index holds our offset; it needs to be loaded
                self.gap_at(after, target)
            }
        };
        assert!(!pos.is_virtual());
        pos
    }

    // Resolve virtual locations to real indexed or gap locations
    pub fn resolve(&self, find: Location, end_of_file: usize) -> Location {
        match find {
            Location::Virtual(loc) => match loc {
                VirtualLocation::Before(0) => Location::Invalid,
                VirtualLocation::Before(offset) => self.locate(TargetOffset::AtOrBefore(offset.min(end_of_file)-1)),
                VirtualLocation::AtOrAfter(offset) => self.locate(TargetOffset::AtOrAfter(offset.min(end_of_file))),
                VirtualLocation::Start => {
                    if let Some(gap) = self.try_gap_at(0, TargetOffset::AtOrAfter(0)) {
                        gap
                    } else {
                        self.get_location(0, 0, TargetOffset::AtOrAfter(0))
                    }
                },
                VirtualLocation::End => {
                    if let Some(gap) = self.try_gap_at(self.indexes.len(), TargetOffset::AtOrBefore(end_of_file.saturating_sub(1))) {
                        gap
                    } else {
                        assert!(!self.indexes.is_empty(), "If it's empty, we should have found a gap");
                        let index = self.indexes.len()-1;
                        let line = self.indexes.last().unwrap().len()-1;
                        let mut pos = self.get_location(index, line, TargetOffset::AtOrBefore(end_of_file));
                        // Skip index at very end of file
                        if pos.found_offset().unwrap() == end_of_file {
                            pos = self.next(pos);
                        }
                        pos
                    }
                },
            },
            _ => find,
        }
    }

    // Resolve the target indexed location, which must already exist
    fn get_location(&self, index: usize, line: usize, target: TargetOffset) -> Location {
        assert!(index < self.indexes.len());
        let j = &self.indexes[index];

        assert!(!j.is_empty());

        let line = line.min(j.len() - 1);

        let offset = j.get(line);
        assert!(offset >= j.start);
        assert!(offset <= j.end);
        Location::Indexed(IndexRef{ index, line , offset, next: target})
    }

    // Find the target near the hinted location
    fn find_location(&self, index: usize, line: usize, target: TargetOffset) -> Location {
        if index < self.indexes.len() && self.indexes[index].is_empty() {
            // No lines recorded in this index; find the gap in the adjacent index
            match target {
                TargetOffset::AtOrAfter(_) => {
                    self.try_gap_at(index + 1, target).unwrap()
                },
                TargetOffset::AtOrBefore(_) => {
                    self.try_gap_at(index, target).unwrap()
                },
            }
        } else {
            let mut pos = self.get_location(index, line, target);
            while pos.is_indexed() && !pos.reached() {
                pos = self.next(pos);
            }
            pos
        }
    }

    // Find index to next/prev line
    pub fn next(&self, find: Location) -> Location {
        if let Location::Indexed(IndexRef{ index, line, offset, next }) = find {
            assert!(index < self.indexes.len());
            let i = &self.indexes[index];
            if line >= i.lines() || i.get(line) != offset {
                // Target location invalidated by changes to self.indexes. Fall back to slow search for line after this offset.
                // panic!("Does this ever happen?");
                self.locate(next.next_exclusive(offset))
            } else {
                let (next_line, gap_index, next_index) = match next {
                    TargetOffset::AtOrAfter(_) => (line + 1, index + 1, index + 1),
                    TargetOffset::AtOrBefore(_) => (line.saturating_sub(1), index, index.saturating_sub(1)),
                };
                if next_line != line && next_line < i.len() {
                    // next line is in the same index
                    self.get_location(index, next_line , next.next_exclusive(offset))
                } else if let Some(gap) = self.try_gap_at(gap_index, next.next_exclusive(offset)) {
                    // next line is not parsed yet
                    gap
                } else if next_index != index {
                    // next line is in the next index
                    self.get_location( next_index, 0 , next.next_exclusive(offset))
                }
                else {
                    // There's no gap before this index, and no lines before it either.  We must be at StartOfFile.
                    assert!(next_index == 0);
                    Location::Invalid
                }
            }
        } else {
            find
        }
    }
}
