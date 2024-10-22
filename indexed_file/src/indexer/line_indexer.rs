// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::SeekFrom;
use crate::files::LogFile;
use crate::indexer::index::Index;
use crate::indexer::eventual_index::{EventualIndex, Location, GapRange, Missing::{Bounded, Unbounded}};
use crate::{LineIndexerIterator, LineViewMode, LogLine, SubLineIterator};

pub trait IndexedLog {
    fn resolve_location(&mut self, pos: Location) -> Location;

    fn read_line_at(&mut self, start: usize) -> std::io::Result<String>;

    fn next_line_index(&self, find: Location) -> Location;

    fn prev_line_index(&self, find: Location) -> Location;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn count_lines(&self) -> usize ;

    // Iterators

    fn iter_offsets(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_
        where Self: Sized {
        self.iter()
    }

    fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        self.iter_view(LineViewMode::WholeLine)
    }

    fn iter_lines_from(&mut self, offset: usize) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        self.iter_view_from(LineViewMode::WholeLine, offset)
    }

    fn iter(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_
    where Self: Sized {

        LineIndexerIterator::new(self)
    }

    fn iter_view(&mut self, mode: LineViewMode) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        SubLineIterator::new(self, mode)
    }

    fn iter_view_from(&mut self, mode: LineViewMode, offset: usize) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        SubLineIterator::new_from(self, mode, offset)
    }

}


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

impl<LOG> LineIndexer<LOG> {

    pub fn new(file: LOG) -> LineIndexer<LOG> {
        Self {
            source: file,
            index: EventualIndex::new(),
        }
    }
}

impl<LOG: LogFile> LineIndexer<LOG> {
    #[inline]
    pub fn wait_for_end(&mut self) {
        self.source.wait_for_end()
    }

    // Resolve virtual locations to already indexed or gap locations
    #[inline]
    fn resolve(&self, find: Location) -> Location {
        self.index.resolve(find, self.len())
    }
}

impl<LOG: LogFile> IndexedLog for LineIndexer<LOG> {
    #[inline]
    fn len(&self) -> usize {
        self.source.len()
    }

    // Read a line at a given offset in the file
    #[inline]
    fn read_line_at(&mut self, start: usize) -> std::io::Result<String> {
        self.source.read_line_at(start)
    }

    // Step to the next indexed line or gap
    #[inline]
    fn next_line_index(&self, find: Location) -> Location {
        self.index.next_line_index(find)
    }

    // Step to the previous indexed line or gap
    #[inline]
    fn prev_line_index(&self, find: Location) -> Location {
        self.index.prev_line_index(find)
    }

    // fill in any gaps by parsing data from the file when needed
    #[inline]
    fn resolve_location(&mut self, pos: Location) -> Location {
        // Resolve any virtuals into gaps or indexed
        let mut pos = self.resolve(pos);

        // Resolve gaps
        while pos.is_gap() {
            pos = self.index_chunk(pos);
        }

        pos
    }

    fn count_lines(&self) -> usize {
        todo!("self.index.count_lines()");
    }
}

impl<LOG: LogFile> LineIndexer<LOG> {
    // Index a chunk of file at some gap location. May index only part of the gap.
    fn index_chunk(&mut self, gap: Location) -> Location {
        // Quench the file in case new data has arrived
        self.source.quench();

        let (target, start, end) = match gap {
            Location::Gap(GapRange { target, index: _, gap: Bounded(start, end) }) => (target, start, end.min(self.len())),
            Location::Gap(GapRange { target, index: _, gap: Unbounded(start) }) => (target, start, self.len()),
            _ => panic!("Tried to index something which is not a gap: {:?}", gap),
        };

        // Offset near where we think we want to read; snapped to gap.
        let offset = target.value().max(start).min(end);
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
}
