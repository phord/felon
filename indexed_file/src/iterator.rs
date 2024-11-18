use crate::indexer::{waypoint::{Position, VirtualPosition}, IndexedLog};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LogLine {
    pub line: String,
    pub offset: usize,
    // pub number: Option<usize>,   // TODO: Relative line number in file;  Future<usize>?
}

impl LogLine {
    pub fn new(line: String, offset: usize) -> Self {
        Self {
            line,
            offset,
        }
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
            log,
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
        }
    }

    pub fn new_from(log: &'a mut LOG, offset: usize) -> Self {
        todo!("replace this with some std::iter method to seek a position quickly");
        let pos = log.seek(offset);
        Self {
            log,
            pos,
            pos_back: Position::Virtual(End),
        }
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next(self.pos.clone());
        self.pos = pos;
        if let Some(line) = line {
            Some(line.offset)
        } else {
            None
        }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos_back, line) = self.log.next_back(self.pos_back.clone());
        self.pos_back = pos_back;
        // todo!("if pos_back < pos, pos = invalid");
        if let Some(line) = line {
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

impl<'a, LOG: IndexedLog> LineIndexerDataIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            log,
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
        }
    }

    pub fn new_from(log: &'a mut LOG, offset: usize) -> Self {
        let pos = log.seek(offset);
        todo!("replace this with some std::iter method to seek a position quickly");
        Self {
            log,
            pos: Position::Virtual(Start),
            pos_back: Position::Virtual(End),
        }
    }
}


impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next_back(self.pos_back.clone());
        self.pos = pos;
        line
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (pos, line) = self.log.next(self.pos.clone());
        self.pos = pos;
        line
    }
}
