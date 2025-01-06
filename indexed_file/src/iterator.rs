use crate::indexer::{
    waypoint::{Position, VirtualPosition},
    IndexedLog,
};
use std::ops::Bound;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LogLine {
    pub line: String,
    pub offset: usize,
    // pub number: Option<usize>,   // TODO: Relative line number in file;  Future<usize>?
}

impl LogLine {
    pub fn new(line: String, offset: usize) -> Self {
        Self { line, offset }
    }
}

impl std::fmt::Display for LogLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: offset?
        write!(f, "{}", self.line)
    }
}

use VirtualPosition::*;

pub struct LineIndexerIterator<'a, LOG> {
    log: &'a mut LOG,
    pos: Position,
    pos_back: Position,
    range: std::ops::Range<usize>,
}

impl<'a, LOG: IndexedLog> LineIndexerIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
            log,
            range: 0..usize::MAX,
        }
    }

    pub fn range<R>(log: &'a mut LOG, offset: R) -> Self
    where
        R: std::ops::RangeBounds<usize>,
    {
        let start = start_offset(offset.start_bound());
        let end = end_offset(offset.end_bound());
        let pos = log.seek(start);
        let pos_back = log.seek(end);
        Self { log, pos, pos_back, range: start..end }
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next(&self.pos);
        self.pos = pos;
        if let Some(line) = line {
            if self.range.contains(&line.offset) {
                self.range = self.range.start.max(line.offset.saturating_add(1))..self.range.end;
                Some(line.offset)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos_back, line) = self.log.next_back(&self.pos_back);
        self.pos_back = pos_back;
        if let Some(line) = line {
            if self.range.contains(&line.offset) {
                self.range = self.range.start..self.range.end.min(line.offset);
                Some(line.offset)
            } else {
                None
            }
        } else {
            None
        }
    }
}

// Iterate over lines as position, string
pub struct LineIndexerDataIterator<'a, LOG: IndexedLog> {
    log: &'a mut LOG,
    pos: Position,
    pos_back: Position,
    range: std::ops::Range<usize>,
}

// returns the byte at the start of our range, inclusive
fn start_offset(bound: Bound<&usize>) -> usize {
    match bound {
        Bound::Included(val) => *val,
        Bound::Excluded(val) => val.saturating_add(1),
        Bound::Unbounded => 0,
    }
}

// End returns the byte after our range, exclusive
fn end_offset(bound: Bound<&usize>) -> usize {
    match bound {
        Bound::Included(val) => val.saturating_add(1),
        Bound::Excluded(val) => *val,
        Bound::Unbounded => usize::MAX,
    }
}

impl<'a, LOG: IndexedLog> LineIndexerDataIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
            log,
            range: 0..usize::MAX,
        }
    }

    pub fn range<R>(log: &'a mut LOG, offset: &'a R) -> Self
    where
        R: std::ops::RangeBounds<usize>,
    {
        let start = start_offset(offset.start_bound());
        let end = end_offset(offset.end_bound());
        let pos = log.seek(start);
        let pos_back = log.seek(end);
        let range = start..end;
        Self { log, pos, pos_back, range}
    }

    fn in_range(&self, line: &LogLine) -> bool {
        // determine if logline overlaps our range
        let line_start = line.offset;
        let line_end = line.offset + line.line.len();
        let range_start = self.range.start;
        let range_end = self.range.end;

        // Line is in range if the start is before range-end (exclusive) and the range-start is before the end (exclusive)
        line_start < range_end && range_start < line_end
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next_back(&self.pos_back);
        if let Some(line) = &line {
            // FIXME: if line is stripped in the future, this range check is wrong.
            if !self.in_range(line) {
                return None;
            }
        }
        self.pos_back = pos;
        line
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next(&self.pos);
        if let Some(line) = &line {
            if !self.in_range(line) {
                return None;
            }
        }
        self.pos = pos;
        line
    }
}
