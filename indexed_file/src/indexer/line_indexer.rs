// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::SeekFrom;
use crate::files::LogFile;
use crate::index::Index;
use crate::eventual_index::{EventualIndex, Location, VirtualLocation, GapRange, Missing::{Bounded, Unbounded}};

pub struct LineIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: EventualIndex,
}

impl<LOG: LogFile> fmt::Debug for LineIndexer<LOG> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineIndexer")
         .field("lines", &self.count_lines())
         .finish()
    }
}

struct LineIndexerIterator<'a, LOG> {
    file: &'a mut LineIndexer<LOG>,
    pos: Location,
    rev_pos: Location,
}

impl<'a, LOG> LineIndexerIterator<'a, LOG> {
    fn new(file: &'a mut LineIndexer<LOG>) -> Self {
        Self {
            file,
            pos: Location::Virtual(VirtualLocation::Start),
            rev_pos: Location::Virtual(VirtualLocation::End),
        }
    }
}

impl<'a, LOG: LogFile> LineIndexerIterator<'a, LOG> {
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
        self.file.source.read_line_at(start)
    }
}

impl<'a, LOG: LogFile> Iterator for LineIndexerIterator<'a, LOG> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.pos);
        self.pos = self.file.index.next_line_index(pos);
        ret
    }
}

impl<'a, LOG: LogFile> DoubleEndedIterator for LineIndexerIterator<'a, LOG> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.rev_pos);
        self.rev_pos = self.file.index.prev_line_index(pos);
        ret
    }
}

// Iterate over lines as position, string
struct LineIndexerDataIterator<'a, LOG> {
    inner: LineIndexerIterator<'a, LOG>,
}

impl<'a, LOG> LineIndexerDataIterator<'a, LOG> {
    fn new(inner: LineIndexerIterator<'a, LOG>) -> Self {
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

impl<LOG: LogFile> LineIndexer<LOG> {

    pub fn new(file: LOG) -> LineIndexer<LOG> {
        Self {
            source: file,
            index: EventualIndex::new(),
        }
    }

    // Resolve virtual locations to real indexed or gap locations
    #[inline]
    fn resolve(&self, find: Location) -> Location {
        self.index.resolve(find, self.source.len())
    }

    // fill in any gaps by parsing data from the file when needed
    fn resolve_location(&mut self, pos: Location) -> Location {
        // Resolve any virtuals into gaps or indexed
        let mut pos = self.resolve(pos);

        // Resolve gaps
        while pos.is_gap() {
            pos = self.index_chunk(pos);
        }

        pos
    }

    // Index a chunk of file at some gap location. May index only part of the gap.
    fn index_chunk(&mut self, gap: Location) -> Location {
        // Quench the file in case new data has arrived
        self.source.quench();

        let (target, start, end) = match gap {
            Location::Gap(GapRange { target, gap: Bounded(start, end) }) => (target, start, end.min(self.source.len())),
            Location::Gap(GapRange { target, gap: Unbounded(start) }) => (target, start, self.source.len()),
            _ => panic!("Tried to index something which is not a gap: {:?}", gap),
        };

        let offset = target.value();
        assert!(start <= offset);
        assert!(end <= self.source.len());

        if start >= end {
            // End of file
            Location::Invalid
        } else {
            let (chunk_start, chunk_end) = self.source.chunk(offset);
            let start = start.max(chunk_start);
            let end = end.min(chunk_end);

            assert!(start <= offset);
            assert!(offset <= end);

            // Send the buffer to the parsers
            self.source.seek(SeekFrom::Start(start as u64)).expect("Seek does not fail");
            let mut index = Index::new();
            index.parse_bufread(&mut self.source, start, end - start).expect("Ignore read errors");
            self.index.merge(index);

            self.index.finalize();
            // FIXME: We don't need to do this binary-search lookup if we know where we hit the gap.  Can Gap carry the hint?
            self.index.locate(target)
        }
    }


    pub fn count_lines(&self) -> usize {
        self.index.lines()
    }

    fn iter(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        LineIndexerIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.iter()
    }

    pub fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = (String, usize)> + '_ {
        LineIndexerDataIterator::new(LineIndexerIterator::new(self))
    }

}
