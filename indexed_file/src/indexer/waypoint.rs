use std::cmp::Ordering;


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

impl Clone for Waypoint {
    fn clone(&self) -> Self {
        match self {
            Waypoint::Mapped(offset) => Waypoint::Mapped(*offset),
            Waypoint::Unmapped(range) => Waypoint::Unmapped(range.clone()),
        }
    }
}

impl Waypoint {
    pub fn cmp_offset(&self) -> usize {
        match self {
            Waypoint::Mapped(offset) => *offset,
            Waypoint::Unmapped(range) => range.start,
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
