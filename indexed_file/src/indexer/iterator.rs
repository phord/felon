use super::LineIndexer;
use crate::{eventual_index::{Location, VirtualLocation}, files::LogFile};


pub(crate) struct LineIndexerIterator<'a, LOG> {
    file: &'a mut LineIndexer<LOG>,
    pos: Location,
    rev_pos: Location,
}

impl<'a, LOG> LineIndexerIterator<'a, LOG> {
    pub(crate) fn new(file: &'a mut LineIndexer<LOG>) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
            rev_pos: Location::Virtual(VirtualLocation::End),
        }
    }
}

impl<'a, LOG: LogFile> LineIndexerIterator<'a, LOG> {
    pub(crate) fn new_from(file: &'a mut LineIndexer<LOG>, offset: usize) -> Self {
        let rev_pos = Location::Virtual(VirtualLocation::Before(offset));
        let pos = Location::Virtual(VirtualLocation::After(offset));
        Self {
            file,
            pos,
            rev_pos,
        }
    }

    fn iterate(&mut self, pos: Location) -> (Location, Option<usize>) {
        let pos = self.file.resolve_location(pos);

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
    fn read_line(&mut self, start: usize) -> std::io::Result<String> {
        self.file.read_line_at(start)
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.pos);
        self.pos = self.file.next_line_index(pos);
        ret
    }
}

impl<'a, LOG: LogFile> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.rev_pos);
        self.rev_pos = self.file.prev_line_index(pos);
        ret
    }
}

// Iterate over lines as position, string
pub(crate) struct LineIndexerDataIterator<'a, LOG> {
    inner: LineIndexerIterator<'a, LOG>,
}

impl<'a, LOG> LineIndexerDataIterator<'a, LOG> {
    pub(crate) fn new(inner: LineIndexerIterator<'a, LOG>) -> Self {
        Self {
            inner,
        }
    }
}

/**
 * TODO: an iterator that iterates lines and builds up the EventualIndex as it goes.
 * TODO: an iterator that iterates from a given line offset forward or reverse.
 *
 * TODO: Can we make a filtered iterator that tests the line in the file buffer and only copy to String if it matches?
 */

impl<'a, LOG: LogFile>  LineIndexerDataIterator<'a, LOG> {
    // Helper function to abstract the wrapping of the inner iterator result
    // If we got a line offset value, read the string and return the Type tuple.
    // TODO: Reuse Self::Type here instead of (String, uszize)
    #[inline]
    fn iterate(&mut self, value: Option<usize>) -> Option<(String, usize)> {
        if let Some(bol) = value {
            // FIXME: Return Some<Result<(offset, String)>> similar to ReadBuf::lines()
            let line = self.inner.read_line(bol).expect("TODO: return Result");
            Some((line, bol))
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

impl<'a, LOG: LogFile> DoubleEndedIterator for LineIndexerDataIterator<'a, LOG> {
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

impl<'a, LOG: LogFile> Iterator for LineIndexerDataIterator<'a, LOG> {
    type Item = (String, usize);

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
