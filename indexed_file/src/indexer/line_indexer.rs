// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::SeekFrom;
use crate::files::LogFile;
use crate::index::Index;
use crate::eventual_index::{EventualIndex, Location, GapRange, Missing::{Bounded, Unbounded}};

use super::{LineIndexerIterator, LineIndexerDataIterator};

pub struct LineIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: EventualIndex,
}

impl<LOG: LogFile> fmt::Debug for LineIndexer<LOG> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineIndexer")
         .finish()
    }
}

impl<LOG: LogFile> LineIndexer<LOG> {

    pub fn new(file: LOG) -> LineIndexer<LOG> {
        Self {
            source: file,
            index: EventualIndex::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.source.len()
    }

    // Resolve virtual locations to already indexed or gap locations
    #[inline]
    fn resolve(&self, find: Location) -> Location {
        self.index.resolve(find, self.len())
    }

    // Read a line at a given offset in the file
    #[inline]
    pub(crate) fn read_line_at(&mut self, start: usize) -> std::io::Result<String> {
        self.source.read_line_at(start)
    }

    // Step to the next indexed line or gap
    #[inline]
    pub(crate) fn next_line_index(&self, find: Location) -> Location {
        self.index.next_line_index(find)
    }

    // Step to the previous indexed line or gap
    #[inline]
    pub(crate) fn prev_line_index(&self, find: Location) -> Location {
        self.index.prev_line_index(find)
    }

    // fill in any gaps by parsing data from the file when needed
    pub(crate) fn resolve_location(&mut self, pos: Location) -> Location {
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
            Location::Gap(GapRange { target, gap: Bounded(start, end) }) => (target, start, end.min(self.len())),
            Location::Gap(GapRange { target, gap: Unbounded(start) }) => (target, start, self.len()),
            _ => panic!("Tried to index something which is not a gap: {:?}", gap),
        };

        let offset = target.value().min(self.len());
        assert!(start <= offset);
        assert!(end <= self.len());

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

    pub fn iter_lines<'a>(&'a mut self) -> impl DoubleEndedIterator<Item = (String, usize)> + 'a {
        LineIndexerDataIterator::new(LineIndexerIterator::new(self))
    }

    pub fn iter_lines_from(&mut self, offset: usize) -> impl DoubleEndedIterator<Item = (String, usize)> + '_ {
        LineIndexerDataIterator::new(LineIndexerIterator::new_from(self, offset))
    }

}
