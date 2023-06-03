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
    pub fn offset(&self) -> Option<usize> {
        match self {
            Location::Indexed(r) => Some(r.offset),
            _ => None,
        }
    }

    #[inline]
    pub fn is_gap(&self) -> bool {
        match self {
            Location::Gap(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_virtual(&self) -> bool {
        match self {
            Location::Virtual(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_indexed(&self) -> bool {
        match self {
            Location::Indexed(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_invalid(&self) -> bool {
        match self {
            Location::Invalid => true,
            _ => false,
        }
    }

    // Get offset in file strictly for comparison in Ord::cmp
    fn relative_offset(&self) -> usize {
        use Location::*;
        use VirtualLocation::*;
        match self {
            Virtual(Start) => 0,
            Virtual(End) => usize::MAX,
            Virtual(Before(off)) => off.saturating_sub(1),
            Virtual(After(off)) => off.saturating_add(1),
            Indexed(iref) => iref.offset,
            Gap(GapRange{target: off, gap: _}) => off.value(),

            Invalid => unreachable!(),
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        self.relative_offset().cmp(&other.relative_offset())
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
    Before(usize),
    After(usize),
}

// The target offset we wanted to reach when filling a gap
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TargetOffset {
    AtOrBefore(usize),
    After(usize),
}

impl TargetOffset {
    pub fn value(&self) -> usize {
        match self {
            TargetOffset::After(x) => *x + 1,
            TargetOffset::AtOrBefore(x) => *x,
        }
    }

    pub fn is_after(&self) -> bool {
        match self {
            TargetOffset::After(_) => true,
            TargetOffset::AtOrBefore(_) => false,
        }
    }

}

// A cursor to some gap in the indexed coverage
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
// Position at `target` is not indexed; need to index region from `gap`
pub struct GapRange {
    // The approximate offset we wanted to reach
    pub target: TargetOffset,

    // The type and size of the gap
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

    pub fn finalize(&mut self) {
        if self.indexes.is_empty() {
            return;
        }

        self.indexes.sort_by_key(|index| index.start);

        let mut prev = 0;
        for index in self.indexes.iter() {
            assert!(index.start >= prev);
            prev = index.end;
        }

        // FIXME: Merge adjacent indexes if one of them is empty
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

    // Describe the gap before the index at pos which includes the target offset
    // If pos is not indexed yet, find the gap at the end of indexes
    // Returns None if there is no gap
    fn try_gap_at(&self, pos: usize, target: TargetOffset) -> Option<Location> {
        use Missing::{Bounded, Unbounded};

        assert!(pos <= self.indexes.len());
        let target_offset = target.value();

        if self.indexes.is_empty() {
            Some(Location::Gap(GapRange { target, gap: Unbounded(0) } ))
        } else if pos == 0 {
            // gap is at start of file
            let next = self.indexes[pos].start;
            if next > 0 {
                assert!(target_offset <= next);
                Some(Location::Gap(GapRange { target, gap: Bounded(0, next) } ))
            } else {
                // There is no gap at start of file
                None
            }
        } else {
            // gap is after index[pos-1]
            let prev_index = &self.indexes[pos-1];
            let prev = prev_index.end;
            if prev_index.contains(&target_offset) {
                // No gap at target_offset
                None
            } else if pos == self.indexes.len() {
                // gap is at end of file; return unbounded range
                assert!(target_offset >= prev);
                Some(Location::Gap(GapRange { target, gap: Unbounded(prev) } ))
            } else {
                // Find the gap between two indexes; bracket result by their [end, start)
                let next = self.indexes[pos].start;
                if next > prev {
                    // assert!(target_offset > prev);
                    // assert!(target_offset < next);
                    Some(Location::Gap(GapRange { target, gap: Bounded(prev, next) } ))
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
        match self.indexes.binary_search_by(|i| i.contains_offset(&offset)) {
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
        }
    }

    // Resolve virtual locations to real indexed or gap locations
    pub fn resolve(&self, find: Location, end_of_file: usize) -> Location {
        match find {
            Location::Virtual(loc) => match loc {
                VirtualLocation::Before(0) => Location::Invalid,
                VirtualLocation::Before(offset) => self.locate(TargetOffset::AtOrBefore(offset-1)),
                VirtualLocation::After(offset) => self.locate(TargetOffset::After(offset)),
                VirtualLocation::Start => {
                    if let Some(gap) = self.try_gap_at(0, TargetOffset::AtOrBefore(0)) {
                        gap
                    } else {
                        self.get_location(0, 0)
                    }
                },
                VirtualLocation::End => {
                    if let Some(gap) = self.try_gap_at(self.indexes.len(), TargetOffset::AtOrBefore(end_of_file.saturating_sub(1))) {
                        gap
                    } else {
                        assert!(!self.indexes.is_empty(), "If it's empty, we should have found a gap");
                        let index = self.indexes.len()-1;
                        let line = self.indexes.last().unwrap().len()-1;
                        let mut pos = self.get_location(index, line);
                        // Skip index at very end of file
                        if pos.offset().unwrap() == end_of_file {
                            pos = self.prev_line_index(pos);
                        }
                        pos
                    }
                },
            },
            _ => find,
        }
    }

    // Resolve the target indexed location, which must already exist
    fn get_location(&self, index: usize, line: usize) -> Location {
        assert!(index < self.indexes.len());
        let j = &self.indexes[index];

        // FIXME: Handle indexes with zero entries
        let line = line.min(j.len() - 1);

        let offset = j.get(line);
        assert!(offset >= j.start);
        assert!(offset <= j.end);
        Location::Indexed(IndexRef{ index, line , offset })
    }

    // Find the target near the hinted location
    fn find_location(&self, index: usize, line: usize, target: TargetOffset) -> Location {
        let mut pos = self.get_location(index, line);
        loop {
            if let Some(p_off) = pos.offset() {
                match target {
                    TargetOffset::After(offset) => {
                        if p_off <= offset {
                            pos = self.next_line_index(pos);
                        } else {
                            break
                        }
                    },
                    TargetOffset::AtOrBefore(offset) => {
                        if p_off > offset {
                            pos = self.prev_line_index(pos);
                        } else {
                            break
                        }
                    },
                }
            } else {
                break
            }
        }
        pos
    }

    // Find index to next line after given index
    pub fn next_line_index(&self, find: Location) -> Location {
        if let Location::Indexed(IndexRef{ index, line, offset}) = find {
            assert!(index < self.indexes.len());
            let i = &self.indexes[index];
            if line >= i.lines() || i.get(line) != offset {
                // Target location invalidated by changes to self.indexes. Fall back to slow search for line after this offset.
                self.locate(TargetOffset::After(offset))
            } else if line + 1 < i.len() {
                // next line is in the same index
                self.get_location( index, line + 1 )
            } else if let Some(gap) = self.try_gap_at(index + 1, TargetOffset::After(i.end)) {
                // next line is not parsed yet
                gap
            } else {
                // next line is in the next index
                self.get_location( index + 1, 0 )
            }
        } else {
            find
        }
    }

    // Find index to prev line before given index
    pub fn prev_line_index(&self, find: Location) -> Location {
        if let Location::Indexed(IndexRef{ index, line, offset}) = find {
            assert!(index < self.indexes.len());
            let i = &self.indexes[index];
            if line >= i.lines() || i.get(line) != offset {
                // Target location invalidated by changes to self.indexes. Fall back to slow search for line before this offset.
                self.locate(TargetOffset::AtOrBefore(offset-1))
            } else if line > 0 {
                // prev line is in the same index
                self.get_location(index, line - 1)
            } else if let Some(gap) = self.try_gap_at(index, TargetOffset::AtOrBefore(self.indexes[index].start)) {
                // prev line is not parsed yet
                gap
            } else if index > 0 {
                // prev line is in the next index
                let j = &self.indexes[index - 1];
                self.get_location(index - 1, j.len() - 1)
            } else {
                // There's no gap before this index, and no lines before it either.  We must be at StartOfFile.
                Location::Invalid
            }
        } else {
            find
        }
    }
}
