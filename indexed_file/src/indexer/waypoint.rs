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
    Existing(IndexIndex, usize, Waypoint),
}

impl Position {
    pub fn new(ndx: IndexIndex, index: &SaneIndex) -> Self {
        let waypoint = index.value(ndx);
        Position::Existing(ndx, waypoint.cmp_offset(), waypoint.clone())
    }
}
// Implement a printer for Position
impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Virtual(virt) => write!(f, "Virtual({:?})", virt),
            Position::Existing(i, target, waypoint) => write!(f, "Existing({:?}, {}, {:?})", i, target, waypoint),
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
        matches!(self, Position::Existing(_, _, Waypoint::Unmapped(_)))
    }

    #[inline]
    // True if this position is at a mapped waypoint.
    pub fn is_mapped(&self) -> bool {
        matches!(self, Position::Existing(_, _, Waypoint::Mapped(_)))
    }

    #[inline]
    pub fn region(&self) -> &Range {
        match self {
            Position::Existing(_, _, waypoint) => waypoint.region(),
            _ => panic!("No range on virtual position"),
        }
    }

    pub(crate) fn clip(&mut self, eof: usize) {
        match self {
            Position::Virtual(VirtualPosition::Offset(offset)) => {
                if *offset >= eof {
                    *self = Position::Virtual(VirtualPosition::Offset(eof.saturating_sub(1)))
                }
            },
            Position::Virtual(VirtualPosition::End) => {
                *self = Position::Virtual(VirtualPosition::Offset(eof.saturating_sub(1)))
            }
            _ => {},
        }
    }

    /// Resolve a virtual position to a real position, or Invalid
    pub(crate) fn resolve(&mut self, index: &SaneIndex) {
        match self {
            Position::Virtual(ref virt) => {
                if let Some(offset) = virt.offset() {
                    let i = index.search(offset);
                    *self = Position::Existing(i, offset, index.value(i).clone());
                } else {
                    *self = Position::Virtual(VirtualPosition::Invalid);
                }
            },
            Position::Existing(i, target, waypoint) => {
                if !index.index_valid(*i) || index.value(*i) != waypoint {
                    log::info!("Waypoint moved; searching new location: {}", target);
                    *self = Position::Virtual(VirtualPosition::Offset(*target));
                    self.resolve(index);
                }
            },
        }
    }

    /// Resolve backwards a virtual position to a real position, or Invalid
    // TODO: dedup this with resolve
    pub(crate) fn resolve_back(&mut self, index: &SaneIndex) {
        match self {
            Position::Virtual(ref virt) => {
                if let Some(offset) = virt.offset() {
                    let mut i = index.search(offset);
                    if !index.index_valid(i) || offset < index.value(i).cmp_offset() {
                        if let Some(ndx) = index.index_prev(i) {
                            i = ndx;
                        }
                    }
                    if index.index_valid(i) {
                        *self = Position::Existing(i, offset, index.value(i).clone());
                    } else {
                        *self = Position::Virtual(VirtualPosition::Invalid);
                    }
                }
            },
            Position::Existing(i, target, waypoint) => {
                if !index.index_valid(*i) || index.value(*i) != waypoint {
                    log::info!("Waypoint moved; searching new location: {}", target);
                    *self = Position::Virtual(VirtualPosition::Offset(*target));
                    self.resolve_back(index);
                }
            },
        }
    }

    /// Extract the waypoint, if there is one
    fn waypoint(&self) -> Option<(Waypoint, usize)> {
        match self {
            Position::Existing(_, target, waypoint) => Some((waypoint.clone(), *target)),
            _ => None,
        }
    }

    pub(crate) fn advance(&mut self, index: &SaneIndex) -> Option<(Waypoint, usize)> {
        if let Position::Existing(i, ..) = self {
            if let Some(next) = index.index_next(*i) {
                let next_waypoint = index.value(next).clone();
                *self = Position::Existing(next, next_waypoint.cmp_offset(), next_waypoint);
            } else {
                *self = Position::Virtual(VirtualPosition::Invalid);
            }
        }
        self.waypoint()
    }

    // If position is virtual, resolve to first appropriate waypoint and return it
    // If it's a waypoint, advance position to the next waypoint and return it
    pub(crate) fn next(&mut self, index: &SaneIndex) -> Option<(Waypoint, usize)> {
        match self {
            Position::Virtual(_) => {
                self.resolve(index);
                // TODO: validate that waypoint is still at index[i]?
                self.waypoint()
            },
            Position::Existing(..) => {
                // Ensure waypoint is still valid
                self.resolve(index);
                // Advance to next waypoint and return it
                self.advance(index)
            },
        }
    }

    pub(crate) fn advance_back(&mut self, index: &SaneIndex) -> Option<(Waypoint, usize)> {
        if let Position::Existing(i, ..) = self {
            if let Some(prev) = index.index_prev(*i) {
                let prev_waypoint = index.value(prev).clone();
                *self = Position::Existing(prev, prev_waypoint.end_offset(), prev_waypoint);
            } else {
                *self = Position::Virtual(VirtualPosition::Invalid);
            }
        }
        self.waypoint()
    }

    // If position is virtual, resolve to first waypoint and return it
    // If it's a waypoint, advance_back position to the prev waypoint and return it
    pub(crate) fn next_back(&mut self, index: &SaneIndex) -> Option<(Waypoint, usize)> {
        match self {
            Position::Virtual(_) => {
                self.resolve_back(index);
                // TODO: validate that waypoint is still at index[i]?
                self.waypoint()
            },
            Position::Existing(..) => {
                // Ensure waypoint is still valid
                self.resolve_back(index);
                // Advance to next waypoint and return it
                self.advance_back(index)
            },
        }
    }

    pub(crate) fn least_offset(&self) -> usize {
        match self {
            Position::Virtual(virt) => virt.offset().unwrap_or(usize::MAX),
            Position::Existing(_, _, waypoint) => waypoint.cmp_offset(),
        }
    }

    /// Is this position still to the left of the other position?
    pub(crate) fn lt(&self, other: &Self) -> bool {
        // If either position is virtual, then it hasn't advanced anything yet.
        if matches!(self, Position::Virtual(_)) || matches!(other, Position::Virtual(_)) {
            return true;
        }
        let left = self.least_offset();
        let right = other.least_offset();
        left < right
    }


}

impl Clone for Waypoint {
    fn clone(&self) -> Self {
        match self {
            Waypoint::Mapped(offset) => Waypoint::Mapped(offset.clone()),
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
    let mut index = SaneIndex::new();
    index.insert(&[0], &(0..13));
    index.insert(&[13], &(13..14));
    index.insert(&[14], &(14..30));
    index.insert(&[30], &(30..51));
    index.insert(&[51], &(51..52));
    index.insert(&[], &(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(),
            vec![Mapped(0..13), Mapped(13..14), Mapped(14..30), Mapped(30..51), Mapped(51..67), Unmapped(67..usize::MAX)]);

    let mut pos = Virtual(Start);
    assert_eq!(pos.next(&index), Some((Mapped(0..13), 0)));
    assert_eq!(pos.next(&index), Some((Mapped(13..14), 13)));
    assert_eq!(pos.next(&index), Some((Mapped(14..30), 14)));
    assert_eq!(pos.next(&index), Some((Mapped(30..51), 30)));
    assert_eq!(pos.next(&index), Some((Mapped(51..67), 51)));
    assert_eq!(pos.next(&index), Some((Unmapped(67..usize::MAX), 67)));
}

#[test]
fn test_position_prev() {
    use Waypoint::*;
    use Position::*;
    use VirtualPosition::*;
    use SaneIndex;
    let mut index = SaneIndex::new();
    index.insert(&[0], &(0..13));
    index.insert(&[13], &(13..14));
    index.insert(&[14], &(14..30));
    index.insert(&[30], &(30..51));
    index.insert(&[51], &(51..52));
    index.insert(&[], &(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(),
            vec![Mapped(0..13), Mapped(13..14), Mapped(14..30), Mapped(30..51), Mapped(51..67), Unmapped(67..usize::MAX)]);

    let mut pos = Virtual(End);
    assert_eq!(pos.next_back(&index), Some((Unmapped(67..usize::MAX), usize::MAX)));
    assert_eq!(pos.next_back(&index), Some((Mapped(51..67), 67)));
    assert_eq!(pos.next_back(&index), Some((Mapped(30..51), 51)));
    assert_eq!(pos.next_back(&index), Some((Mapped(14..30), 30)));
    assert_eq!(pos.next_back(&index), Some((Mapped(13..14), 14)));
    assert_eq!(pos.next_back(&index), Some((Mapped(0..13), 13)));
}

#[test]
fn test_position_prev_unmapped() {
    use Waypoint::*;
    use Position::*;
    use VirtualPosition::*;
    use SaneIndex;
    let mut index = SaneIndex::new();
    index.insert(&[0], &(0..13));
    index.insert(&[13], &(13..14));
    index.insert(&[14], &(14..30));
    index.insert(&[30], &(30..51));
    index.insert(&[51], &(51..52));
    index.insert(&[], &(52..67));
    assert_eq!(index.iter().collect::<Vec<_>>(),
            vec![Mapped(0..13), Mapped(13..14), Mapped(14..30), Mapped(30..51), Mapped(51..67), Unmapped(67..usize::MAX)]);

    let mut pos = Virtual(End);
    assert_eq!(pos.next_back(&index), Some((Unmapped(67..usize::MAX), usize::MAX)));
    assert_eq!(pos.next_back(&index), Some((Mapped(51..67), 67)));
    assert_eq!(pos.next_back(&index), Some((Mapped(30..51), 51)));
    assert_eq!(pos.next_back(&index), Some((Mapped(14..30), 30)));
    assert_eq!(pos.next_back(&index), Some((Mapped(13..14), 14)));
    assert_eq!(pos.next_back(&index), Some((Mapped(0..13), 13)));
}
