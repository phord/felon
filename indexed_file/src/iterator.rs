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
}

impl<'a, LOG: IndexedLog> LineIndexerIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
            log,
        }
    }

    pub fn range<R>(log: &'a mut LOG, offset: R) -> Self
    where
        R: std::ops::RangeBounds<usize>,
    {
        let pos = log.seek(value_or(offset.start_bound(), 0));
        let pos_back = log.seek(value_or(offset.end_bound(), usize::MAX));
        Self { log, pos, pos_back }
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next(self.pos.clone());
        self.pos = pos;
        if !self.pos.lt(&self.pos_back) {
            None
        } else if let Some(line) = line {
            Some(line.offset)
        } else {
            // FIXME: invalidate iterators?
            None
        }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos_back, line) = self.log.next_back(self.pos_back.clone());
        self.pos_back = pos_back;
        if !self.pos.lt(&self.pos_back) {
            None
        } else if let Some(line) = line {
            Some(line.offset)
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
}

fn value_or(bound: Bound<&usize>, def: usize) -> usize {
    match bound {
        Bound::Included(val) => *val,
        Bound::Excluded(val) => val.saturating_sub(1), // FIXME: How to handle ..0?
        Bound::Unbounded => def,
    }
}

impl<'a, LOG: IndexedLog> LineIndexerDataIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
            log,
        }
    }

    pub fn range<R>(log: &'a mut LOG, offset: &'a R) -> Self
    where
        R: std::ops::RangeBounds<usize>,
    {
        let pos = log.seek(value_or(offset.start_bound(), 0));
        let pos_back = log.seek(value_or(offset.end_bound(), usize::MAX));
        Self { log, pos, pos_back }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if !self.pos.lt(&self.pos_back) {
            return None;
        }
        let (pos, line) = self.log.next_back(self.pos_back.clone());
        self.pos_back = pos;
        line
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if !self.pos.lt(&self.pos_back) {
            return None;
        }
        let (pos, line) = self.log.next(self.pos.clone());
        self.pos = pos;
        line
    }
}
