use crate::{indexer::{eventual_index::{Location, VirtualLocation}}, Log};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
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


pub struct LineIndexerIterator<'a> {
    log: &'a mut Log,
    pos: Location,
    rev_pos: Location,
}

impl<'a> LineIndexerIterator<'a> {
    pub fn new(log: &'a mut Log) -> Self {
        Self {
            log,
            pos: Location::Virtual(VirtualLocation::Start),
            rev_pos: Location::Virtual(VirtualLocation::End),
        }
    }
}

impl<'a> LineIndexerIterator<'a> {
    pub fn new_from(log: &'a mut Log, offset: usize) -> Self {
        let rev_pos = Location::Virtual(VirtualLocation::Before(offset));
        let pos = Location::Virtual(VirtualLocation::After(offset));
        Self {
            log,
            pos,
            rev_pos,
        }
    }

    // helper: resolves pos into a location in the file, but does not actually "move" the iterator
    fn iterate(&mut self, pos: Location) -> (Location, Option<usize>) {
        let pos = self.log.file.resolve_location(pos);

        let ret = pos.offset();
        if self.rev_pos == self.pos {
            // End of iterator when fwd and rev meet
            self.rev_pos = Location::Invalid;
            self.pos = Location::Invalid;
            (Location::Invalid, ret)
        } else {
            (pos, ret)
        }
    }

    // Read a string at a given start from our log source
    #[inline]
    fn read_line(&mut self, offset: usize) -> std::io::Result<LogLine> {
        let line = self.log.file.read_line_at(offset)?;
        Ok(LogLine::new( line, offset ))
    }
}

impl<'a> Iterator for LineIndexerIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.pos);
        self.pos = self.log.file.next_line_index(pos);
        if ret.is_some() && ret.unwrap() >= self.log.file.len() {
            None
        } else {
            ret
        }
    }
}

impl<'a> DoubleEndedIterator for LineIndexerIterator<'a> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.rev_pos);
        self.rev_pos = self.log.file.prev_line_index(pos);
        ret
    }
}

// Iterate over lines as position, string
pub struct LineIndexerDataIterator<'a> {
    inner: LineIndexerIterator<'a>,
}

impl<'a> LineIndexerDataIterator<'a> {
    pub fn new(log: &'a mut Log) -> Self {
        let inner = LineIndexerIterator::new(log);
        Self {
            inner,
        }
    }

    pub fn new_from(log: &'a mut Log, offset: usize) -> Self {
        let inner = LineIndexerIterator::new_from(log, offset);
        Self {
            inner,
        }
    }
}

/**
 * TODO: an iterator that iterates lines and builds up the EventualIndex as it goes.
 *
 * TODO: Can we make a filtered iterator that tests the line in the file buffer and only copy to String if it matches?
 */

impl<'a>  LineIndexerDataIterator<'a> {
    // Helper function to abstract the wrapping of the inner iterator result
    // If we got a line offset value, read the string and return the Type tuple.
    // TODO: Reuse Self::Type here instead of respecifying LogLine
    #[inline]
    fn iterate(&mut self, value: Option<usize>) -> Option<LogLine> {
        if let Some(bol) = value {
            // FIXME: Return Some<Result<(offset, String)>> similar to ReadBuf::lines()
            let line = self.inner.read_line(bol).expect("TODO: return Result");
            Some(line)
        } else {
            None
        }
    }

    // Advance backwards without reading lines into strings
    #[inline]
    fn advance_back_by(&mut self, n: usize) -> Result<(), usize> {
        for i in 0..n {
            self.inner.next_back().ok_or(i)?;
        }
        Ok(())
    }

    // Advance without reading lines into strings
    #[inline]
    fn advance_by(&mut self, n: usize) -> Result<(), usize> {
        for i in 0..n {
            self.inner.next().ok_or(i)?;
        }
        Ok(())
    }
}

impl<'a> DoubleEndedIterator for LineIndexerDataIterator<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let ret = self.inner.next_back();
        self.iterate(ret)
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.advance_back_by(n).ok()?;
        self.next_back()
    }
}

impl<'a> Iterator for LineIndexerDataIterator<'a> {
    type Item = LogLine;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.inner.next();
        self.iterate(ret)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.advance_by(n).ok()?;
        self.next_back()
    }
}
