use crate::indexer::{eventual_index::{Location, VirtualLocation}, line_indexer::{IndexedLog, IndexedLogOld, LogLocation}};

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


pub struct LineIndexerIterator<'a, LOG> {
    log: &'a mut LOG,
    pos: LogLocation,
    rev_pos: LogLocation,
}

impl<'a, LOG: IndexedLog> LineIndexerIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: log.seek(0),
            rev_pos: log.seek(usize::MAX),
            log,
        }
    }
}

impl<'a, LOG: IndexedLog> LineIndexerIterator<'a, LOG> {
    pub fn new_from(log: &'a mut LOG, offset: usize) -> Self {
        let rev_pos = log.seek(offset);
        let pos = log.seek(offset);
        Self {
            log,
            pos,
            rev_pos,
        }
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        for i in 0..5 {
            // We should have resolved it by now
            assert!(i<4);
            let (line, pos) = self.log.next(&self.pos);
            self.pos = pos;
            if let Some(line) = line {
                return Some(line.offset)
            }
        }
        None
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        for i in 0..5 {
            // We should have resolved it by now
            assert!(i<4);
            let (line, pos) = self.log.next_back(&self.rev_pos);
            self.rev_pos = pos;
            if let Some(line) = line {
                return Some(line.offset)
            }
        }
        None
    }
}

// Iterate over lines as position, string
pub struct LineIndexerDataIterator<'a, LOG: IndexedLog> {
    log: &'a mut LOG,
    pos: LogLocation,
    rev_pos: LogLocation,
}

impl<'a, LOG: IndexedLog> LineIndexerDataIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: log.seek(0),
            rev_pos: log.seek(usize::MAX),
            log,
        }
    }

    pub fn new_from(log: &'a mut LOG, offset: usize) -> Self {
        let rev_pos = log.seek(offset);
        let pos = log.seek(offset);
        Self {
            log,
            pos,
            rev_pos,
        }
    }
}


impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        for i in 0..5 {
            // We should have resolved it by now
            assert!(i<4);
            let (line, pos) = self.log.next_back(&self.rev_pos);
            self.rev_pos = pos;
            if let Some(line) = line {
                return Some(line)
            } else if let Some(pos) = self.pos.tracker {
                if pos.is_invalid() {
                    return None
                }
            }
        }
        None
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        for i in 0..5 {
            // dbg!(&self.pos);

            // We should have resolved it by now
            assert!(i<4);
            let (line, pos) = self.log.next(&self.pos);
            self.pos = pos;
            if let Some(line) = line {
                return Some(line)
            } else if let Some(pos) = self.pos.tracker {
                if pos.is_invalid() {
                    return None
                }
            }
        }
        None
    }
}
