use std::cmp::Ordering;

use super::sane_index::{IndexIndex, SaneIndex};


type Range = std::ops::Range<usize>;

#[derive(Debug, PartialEq, Eq)]
pub enum Waypoint {
    /// A line we have seen before; End of one waypoint equals the beginning of the next.
    Mapped(Range),

    /// An uncharted region; beware of index shift. if we find \n at 0, the next line starts at 1.
    /// Range bytes we have to search is in [start, end)
    /// Range of Mapped we may discover is in (start, end]
    Unmapped(Range),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualPosition {
    /// Start of file
    Start,

    /// End of file
    End,

    /// Invalid iterator (exhausted)
    Invalid,

    /// Offset in the file
    Offset(usize),
}

impl VirtualPosition {
    pub fn offset(&self) -> Option<usize> {
        match self {
            VirtualPosition::Offset(offset) => Some(*offset),
            VirtualPosition::Start => Some(0),
            VirtualPosition::End => Some(usize::MAX),
            VirtualPosition::Invalid => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    /// Some unresolved position
    Virtual(VirtualPosition),

    /// A specific waypoint that exists (or existed) in the file
    /// (IndexIndex, Waypoint we found at IndexIndex)
    Existing(IndexIndex, Waypoint),
}

impl Default for Position {
    fn default() -> Self {
        Position::invalid()
    }
}

impl Position {
    pub fn new(ndx: IndexIndex, index: &SaneIndex) -> Self {
        let (i,j) = ndx;
        if i == index.index.len() && j == 0 {
            return Position::Virtual(VirtualPosition::End);
        } else if i > index.index.len() || j >= index.index[i].len() {
            return Position::invalid();
        }
        let waypoint = index.value(ndx);
        Position::Existing(ndx, waypoint.clone())
    }

    #[inline]
    pub fn invalid() -> Self {
        Position::Virtual(VirtualPosition::Invalid)
    }

    #[inline]
    pub fn from(offset: usize) -> Self {
        Position::Virtual(VirtualPosition::Offset(offset))
    }
}
// Implement a printer for Position
impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Virtual(virt) => write!(f, "Virtual({:?})", virt),
            Position::Existing(i, waypoint) => write!(f, "Existing({:?}, {:?})", i, waypoint),
        }
    }
}

impl Position {
    #[inline]
    pub fn is_invalid(&self) -> bool {
        matches!(self, Position::Virtual(VirtualPosition::Invalid))
    }

    // True if this position is at an unmapped waypoint.
    // False if virtual or mapped.
    #[inline]
    pub fn is_unmapped(&self) -> bool {
        matches!(self, Position::Existing(_, Waypoint::Unmapped(_)))
    }

    #[inline]
    // True if this position is at a mapped waypoint.
    pub fn is_mapped(&self) -> bool {
        matches!(self, Position::Existing(_, Waypoint::Mapped(_)))
    }

    #[inline]
    pub fn is_virtual(&self) -> bool {
        matches!(self, Position::Virtual(_))
    }

    #[inline]
    pub fn region(&self) -> &Range {
        match self {
            Position::Existing(_, waypoint) => waypoint.region(),
            _ => panic!("No range on virtual position"),
        }
    }

    /// Resolve a virtual position to a real position, or Invalid
    pub(crate) fn resolve(&self, index: &SaneIndex) -> Position{
        match self {
            Position::Virtual(ref virt) => {
                if let Some(offset) = virt.offset() {
                    let i = index.search(offset);
                    if index.index_valid(i) {
                        Position::Existing(i, index.value(i).clone())
                    } else {
                        Position::invalid()
                    }
                } else {
                    Position::invalid()
                }
            },
            Position::Existing(i, waypoint) => {
                if !index.index_valid(*i) || index.value(*i) != waypoint {
                    log::info!("Waypoint moved; searching new location: {}", self);
                    Position::Virtual(VirtualPosition::Offset(self.least_offset())).resolve(index)
                } else {
                    self.clone()
                }
            },
        }
    }

    /// Resolve backwards a virtual position to a real position, or Invalid
    // TODO: dedup this with resolve
    pub(crate) fn resolve_back(&self, index: &SaneIndex) -> Position {
        match self {
            Position::Virtual(ref virt) => {
                if let Some(offset) = virt.offset() {
                    let mut i = index.search(offset);
                    // offset is exclusive when seeking backwards.  If we found a waypoint at offset, we need to step back one.
                    if !index.index_valid(i) || offset <= index.value(i).cmp_offset() {
                        if let Some(ndx) = index.index_prev(i) {
                            i = ndx;
                        }
                    }
                    if index.index_valid(i) {
                        Position::Existing(i, index.value(i).clone())
                    } else {
                        Position::invalid()
                    }
                } else {
                    Position::invalid()
                }
            },
            Position::Existing(i, waypoint) => {
                if !index.index_valid(*i) || index.value(*i) != waypoint {
                    log::info!("Waypoint moved; searching new location: {}", self);
                    Position::Virtual(VirtualPosition::Offset(self.least_offset())).resolve_back(index)
                } else {
                    self.clone()
                }
            },
        }
    }

    /// Extract the waypoint if there is one
    #[cfg(test)]
    pub(crate) fn waypoint(&self) -> Option<Waypoint> {
        match self {
            Position::Existing(_, waypoint) => Some(waypoint.clone()),
            _ => None,
        }
    }

    /// Move this position forward on the index, returning the new waypoint
    pub(crate) fn advance(&self, index: &SaneIndex) -> Position {
        if let Position::Existing(i, ..) = self {
            if let Some(next) = index.index_next(*i) {
                let next_waypoint = index.value(next).clone();
                Position::Existing(next, next_waypoint)
            } else {
                Position::invalid()
            }
        } else {
            Position::invalid()
        }
    }

    /// Find the next previous Position from this one
    pub(crate) fn advance_back(&self, index: &SaneIndex) -> Position {
        if let Position::Existing(i, ..) = self {
            if let Some(prev) = index.index_prev(*i) {
                let prev_waypoint = index.value(prev).clone();
                Position::Existing(prev, prev_waypoint)
            } else {
                Position::invalid()
            }
        } else {
            Position::invalid()
        }

    }

    // Advance the position to the next waypoint
    pub(crate) fn next(&self, index: &SaneIndex) -> Position {
        self.resolve(index).advance(index)
    }

    // If position is virtual, resolve to appropriate waypoint and return it
    // If it's a waypoint, advance_back position to the prev waypoint and return it
    pub(crate) fn next_back(&self, index: &SaneIndex) -> Position {
        self.resolve_back(index)
            .advance_back(index)
    }

    /// Start of line if Position points to an existing line, else None
    pub fn offset(&self) -> Option<usize> {
        if self.is_mapped() {
            Some(self.least_offset())
        } else {
            None
        }
    }

    /// start
    pub(crate) fn least_offset(&self) -> usize {
        match self {
            Position::Virtual(virt) => virt.offset().unwrap_or(usize::MAX),
            Position::Existing(_, waypoint) => waypoint.cmp_offset(),
        }
    }

    /// end
    pub(crate) fn most_offset(&self) -> usize {
        match self {
            Position::Virtual(virt) => virt.offset().unwrap_or(0),
            Position::Existing(_, waypoint) => waypoint.end_offset(),
        }
    }
}

impl Clone for Waypoint {
    fn clone(&self) -> Self {
        match self {
            Waypoint::Mapped(range) => Waypoint::Mapped(range.clone()),
            Waypoint::Unmapped(range) => Waypoint::Unmapped(range.clone()),
        }
    }
}

impl Waypoint {
    fn region(&self) -> &Range {
        match self {
            Waypoint::Mapped(range) => range,
            Waypoint::Unmapped(range) => range,
        }
    }

    // Offset used for sorting
    pub fn cmp_offset(&self) -> usize {
        self.region().start
    }

    // End of the waypoint range (inclusive)
    pub fn end_offset(&self) -> usize {
        self.region().end
    }

    pub fn contains(&self, offset: usize) -> bool {
        self.region().contains(&offset)
    }

    pub fn is_mapped(&self) -> bool {
        matches!(self, Waypoint::Mapped(_))
    }

    pub fn split_at(&self, offset: usize) -> (Option<Waypoint>, Option<Waypoint>) {
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


#[test]
fn test_waypoint_cmp() {
    use Waypoint::*;
    assert_eq!(Mapped(0..1).cmp(&Mapped(0..1)), Ordering::Equal);
    assert_eq!(Mapped(0..1).cmp(&Mapped(1..2)), Ordering::Less);
    assert_eq!(Mapped(1..2).cmp(&Mapped(0..1)), Ordering::Greater);
}

#[test]
fn test_waypoint_cmp_unmapped() {
    use Waypoint::*;
    assert_eq!(Unmapped(0..1).cmp(&Unmapped(0..1)), Ordering::Equal);
    assert_eq!(Unmapped(0..1).cmp(&Unmapped(1..2)), Ordering::Less);
    assert_eq!(Unmapped(1..2).cmp(&Unmapped(0..1)), Ordering::Greater);
}

#[test]
fn test_waypoint_cmp_mixed() {
    use Waypoint::*;
    assert_eq!(Mapped(0..1).cmp(&Unmapped(0..1)), Ordering::Less);
    assert_eq!(Unmapped(0..1).cmp(&Mapped(0..1)), Ordering::Greater);
}

#[test]
fn test_position_next() {
    use Waypoint::*;
    use Position::*;
    use VirtualPosition::*;
    use SaneIndex;
    let mut index = SaneIndex::default();
    index.insert(&(0..13));
    index.insert(&(13..14));
    index.insert(&(30..51));
    index.insert(&(51..52));
    index.erase(&(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(),
            vec![Mapped(0..13), Mapped(13..14), Unmapped(14..30), Mapped(30..51), Mapped(51..52), Unmapped(67..usize::MAX)]);

    let pos = Virtual(Start);
    let pos = pos.resolve(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(0..13)));
    let pos = pos.next(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(13..14)));
    let pos = pos.next(&index);
    assert_eq!(pos.waypoint(), Some(Unmapped(14..30)));
    let pos = pos.next(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(30..51)));
    let pos = pos.next(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(51..52)));
    let pos = pos.next(&index);
    assert_eq!(pos.waypoint(), Some(Unmapped(67..usize::MAX)));
}

#[test]
fn test_position_prev() {
    use Waypoint::*;
    use Position::*;
    use VirtualPosition::*;
    use SaneIndex;
    let mut index = SaneIndex::default();
    index.insert(&(0..13));
    index.insert(&(13..14));
    index.insert(&(30..51));
    index.insert(&(51..52));
    index.erase(&(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(),
            vec![Mapped(0..13), Mapped(13..14), Unmapped(14..30), Mapped(30..51), Mapped(51..52), Unmapped(67..usize::MAX)]);

    let pos = Virtual(End);
    let pos = pos.resolve_back(&index);
    assert_eq!(pos.waypoint(), Some(Unmapped(67..usize::MAX)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(51..52)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(30..51)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Unmapped(14..30)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(13..14)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(0..13)));
}

#[test]
fn test_position_prev_unmapped() {
    use Waypoint::*;
    use Position::*;
    use VirtualPosition::*;
    use SaneIndex;
    let mut index = SaneIndex::default();
    index.insert(&(0..13));
    index.insert(&(13..14));
    index.insert(&(30..51));
    index.insert(&(51..52));
    index.erase(&(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(),
            vec![Mapped(0..13), Mapped(13..14), Unmapped(14..30), Mapped(30..51), Mapped(51..52), Unmapped(67..usize::MAX)]);

    let pos = Virtual(End);
    let pos = pos.resolve_back(&index);
    assert_eq!(pos.waypoint(), Some(Unmapped(67..usize::MAX)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(51..52)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(30..51)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Unmapped(14..30)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(13..14)));
    let pos = pos.next_back(&index);
    assert_eq!(pos.waypoint(), Some(Mapped(0..13)));
}
