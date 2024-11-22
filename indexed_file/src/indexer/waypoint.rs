use std::cmp::Ordering;

use super::sane_index::SaneIndex;


type Range = std::ops::Range<usize>;

#[derive(Debug, PartialEq, Eq)]
pub enum Waypoint {
    /// The start of a line; e.g., first line starts at 0
    Mapped(usize),

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
    Some(usize, Waypoint),
}

impl Position {
    #[inline]
    pub fn is_invalid(&self) -> bool {
        matches!(self, Position::Virtual(VirtualPosition::Invalid))
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
                    if i < index.index.len() {
                        *self = Position::Some(i, index.index[i].clone());
                    } else {
                        *self = Position::Virtual(VirtualPosition::Invalid);
                    }
                }
            },
            Position::Some(i, waypoint) => {
                if *i >= index.index.len() || index.index[*i] != *waypoint {
                    log::info!("Waypoint moved; searching new location: {}", waypoint.cmp_offset());
                    *self = Position::Virtual(VirtualPosition::Offset(waypoint.cmp_offset()));
                    self.resolve(index);
                }
            },
        }
    }

    /// Resolve backwards a virtual position to a real position, or Invalid
    // TODO: dedup this with resolve
    pub(crate) fn resolve_back(&mut self, index: &SaneIndex) {
        // dbg!(&self);
        match self {
            Position::Virtual(ref virt) => {
                // dbg!(&virt);
                if let Some(offset) = virt.offset() {
                    // dbg!(offset);
                    let i = index.search(offset);
                    // dbg!(i);
                    let i =
                        if i == index.index.len() || offset < index.index[i].cmp_offset() {
                            i.saturating_sub(1)
                        } else {
                            i
                        };
                    if i < index.index.len() {
                        *self = Position::Some(i, index.index[i].clone());
                    } else {
                        *self = Position::Virtual(VirtualPosition::Invalid);
                    }
                }
            },
            Position::Some(i, waypoint) => {
                if *i >= index.index.len() || index.index[*i] != *waypoint {
                    log::info!("Waypoint moved; searching new location: {}", waypoint.cmp_offset());
                    *self = Position::Virtual(VirtualPosition::Offset(waypoint.cmp_offset()));
                    self.resolve_back(index);
                }
            },
        }
        // dbg!(&self);
    }

    fn waypoint(&self) -> Option<Waypoint> {
        match self {
            Position::Some(_, waypoint) => Some(waypoint.clone()),
            _ => None,
        }
    }

    pub(crate) fn advance(&mut self, index: &SaneIndex) -> Option<Waypoint> {
        if let Position::Some(i, _) = self {
            let next = *i + 1;
            if next < index.index.len() {
                let next_waypoint = index.index[next].clone();
                *self = Position::Some(next, next_waypoint);
            }
        }
        return self.waypoint();
    }

    // If position is virtual, resolve to first waypoint and return it
    // If it's a waypoint, advance position to the next waypoint and return it
    pub(crate) fn next(&mut self, index: &SaneIndex) -> Option<Waypoint> {
        match self {
            Position::Virtual(_) => {
                self.resolve(index);
                // TODO: validate that waypoint is still at index[i]?
                return self.waypoint();
            },
            Position::Some(..) => {
                // Ensure waypoint is still valid
                self.resolve(index);
                // Advance to next waypoint and return it
                return self.advance(index);
            },
        }
    }

    pub(crate) fn advance_back(&mut self, index: &SaneIndex) -> Option<Waypoint> {
        if let Position::Some(i, _) = self {
            if *i > 0 {
                let next = *i - 1;
                let next_waypoint = index.index[next].clone();
                *self = Position::Some(next, next_waypoint);
            } else {
                *self = Position::Virtual(VirtualPosition::Invalid);
            }
        }
        return self.waypoint();
    }

    // If position is virtual, resolve to first waypoint and return it
    // If it's a waypoint, advance_back position to the prev waypoint and return it
    pub(crate) fn next_back(&mut self, index: &SaneIndex) -> Option<Waypoint> {
        match self {
            Position::Virtual(_) => {
                self.resolve_back(index);
                // TODO: validate that waypoint is still at index[i]?
                return self.waypoint();
            },
            Position::Some(..) => {
                // Ensure waypoint is still valid
                self.resolve_back(index);
                // Advance to next waypoint and return it
                return self.advance_back(index);
            },
        }
    }

    fn least_offset(&self) -> usize {
        match self {
            Position::Virtual(virt) => virt.offset().unwrap_or(usize::MAX),
            Position::Some(_, waypoint) => waypoint.cmp_offset(),
        }
    }

    fn most_offset(&self) -> usize {
        match self {
            Position::Virtual(virt) => virt.offset().unwrap_or(usize::MAX),
            Position::Some(_, waypoint) => waypoint.end_offset(),
        }
    }

    /// Is this position still to the left of the other position?
    pub(crate) fn lt(&self, other: &Self) -> bool {
        let left = self.least_offset();
        let right = other.most_offset();
        left < right
    }


}

impl Clone for Waypoint {
    fn clone(&self) -> Self {
        match self {
            Waypoint::Mapped(offset) => Waypoint::Mapped(*offset),
            Waypoint::Unmapped(range) => Waypoint::Unmapped(range.clone()),
        }
    }
}

impl Waypoint {
    // Offset used for sorting
    pub fn cmp_offset(&self) -> usize {
        match self {
            Waypoint::Mapped(offset) => *offset,
            Waypoint::Unmapped(range) => range.start,
        }
    }

    // End of the waypoint range (inclusive)
    pub fn end_offset(&self) -> usize {
        match self {
            Waypoint::Mapped(offset) => *offset,
            Waypoint::Unmapped(range) => range.end,
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
