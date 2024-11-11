use crate::indexer::IndexedLog;

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
    pos: usize,
    rev_pos: usize,
}

impl<'a, LOG: IndexedLog> LineIndexerIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: 0,
            rev_pos: usize::MAX,
            log,
        }
    }

    pub fn new_from(log: &'a mut LOG, offset: usize) -> Self {
        let rev_pos = offset;
        let pos = offset;
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
        if self.pos == self.rev_pos {
            return None
        }
        self.log.seek(self.pos);
        if let Some(line) = self.log.next() {
            self.pos = line.offset;
            Some(line.offset)
        } else {
            self.pos = self.rev_pos;
            None
        }
    }
}

impl<'a, LOG: IndexedLog> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.pos == self.rev_pos {
            return None
        }
        self.log.seek(self.rev_pos);
        if let Some(line) = self.log.next_back() {
            self.rev_pos = line.offset;
            Some(line.offset)
        } else {
            self.rev_pos = self.pos;
            None
        }
    }
}

// Iterate over lines as position, string
pub struct LineIndexerDataIterator<'a, LOG: IndexedLog> {
    log: &'a mut LOG,
    pos: usize,
    rev_pos: usize,
}

impl<'a, LOG: IndexedLog> LineIndexerDataIterator<'a, LOG> {
    pub fn new(log: &'a mut LOG) -> Self {
        Self {
            pos: 0,
            rev_pos: usize::MAX,
            log,
        }
    }

    pub fn new_from(log: &'a mut LOG, offset: usize) -> Self {
        let rev_pos = offset;
        let pos = offset;
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
        if self.pos == self.rev_pos {
            return None
        }
        // self.log.seek(self.rev_pos);
        if let Some(line) = self.log.next_back() {
            self.rev_pos = line.offset;
            Some(line)
        } else {
            self.rev_pos = self.pos;
            None
        }
    }
}

impl<'a, LOG: IndexedLog> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.rev_pos || self.pos >= self.log.len() {
            return None
        }
        // self.log.seek(self.pos);
        if let Some(line) = self.log.next() {
            self.pos = line.offset + line.line.len();
            Some(line)
        } else {
            self.pos = self.rev_pos;
            None
        }
    }
}
