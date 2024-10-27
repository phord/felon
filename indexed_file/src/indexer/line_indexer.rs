// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::SeekFrom;
use std::ops::Range;
use crate::files::LogFile;
use crate::indexer::index::Index;
use crate::indexer::eventual_index::{EventualIndex, Location, GapRange, Missing::{Bounded, Unbounded}};
use crate::{LineIndexerIterator, LineViewMode, LogLine, SubLineIterator};
use super::eventual_index::VirtualLocation;

type LogRange = Range<usize>;

#[derive(Debug)]
pub struct LogLocation {
    // The range we scanned to return the current line
    pub range: LogRange,

    // The location of the next line to read
    pub tracker: Location,
}

pub trait IndexedLog {
    /// Generate a cursor to use for reading lines from the file
    fn seek(&self, pos: usize) -> LogLocation {
        LogLocation {
            range: pos..pos,
            tracker: Location::Virtual(VirtualLocation::AtOrAfter(pos)),
        }
    }

    /// Generate a cursor to use for reading lines from the file
    fn seek_rev(&self, pos: usize) -> LogLocation {
        LogLocation {
            range: pos..pos,
            tracker: Location::Virtual(VirtualLocation::Before(pos)),
        }
    }

    // Read the line at pos, if any, and return the iterator results and the new cursor
    fn read_line(&mut self, pos: &mut LogLocation, next_pos: Location) -> Option<LogLine>;

    /// Read the next line from the file
    /// returns search results and the new cursor
    /// If line is None and pos.tracker is Some(Invalid), we're at the start of the file
    /// If line is None and tracker is anything else, there may be more to read
    fn next(&mut self, pos: &mut LogLocation) -> Option<LogLine>;

    /// Read the previous line from the file
    /// returns search results and the new cursor
    /// If line is None and pos.tracker is Some(Invalid), we're at the start of the file
    /// If line is None and tracker is anything else, there may be more to read
    fn next_back(&mut self, pos: &mut LogLocation) -> Option<LogLine>;


    // Iterators

    // TEST ONLY
    fn iter_offsets(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_
        where Self: Sized {
        self.iter()
    }

    // TEST ONLY - Called from iter_offsets
    fn iter(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_
    where Self: Sized {

        LineIndexerIterator::new(self)
    }

    // TEST and MergedLog
    fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        self.iter_view(LineViewMode::WholeLine)
    }

    // Used in FilteredLog to stream from inner
    fn iter_lines_from(&mut self, offset: usize) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        self.iter_view_from(LineViewMode::WholeLine, offset)
    }

    // TEST and MergedLog
    fn iter_view(&mut self, mode: LineViewMode) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        SubLineIterator::new(self, mode)
    }

    // Used in FilteredLog and Document (grok)
    fn iter_view_from(&mut self, mode: LineViewMode, offset: usize) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        SubLineIterator::new_from(self, mode, offset)
    }

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn count_lines(&self) -> usize ;

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

    // fill in any gaps by parsing data from the file when needed
    #[inline]
    // FIXME: Make this private
    pub fn resolve_location(&mut self, pos: Location) -> Location {
        // Resolve any virtuals into gaps or indexed
        let mut pos = self.index.resolve(pos, self.len());

        // Resolve gaps
        for _ in 0..10 {
            if !pos.is_gap() { return pos; }
            pos = self.index_chunk(pos);
        }
        assert!(!pos.is_gap());
        pos
    }
}

impl<LOG: LogFile> IndexedLog for LineIndexer<LOG> {

    fn read_line(&mut self, pos: &mut LogLocation, next_pos: Location) -> Option<LogLine> {
        if pos.tracker.is_invalid() {
            return None;
        }
        let origin = pos.tracker.offset().min(self.source.len());
        if origin >= self.source.len() {
            pos.tracker = Location::Invalid;
            return None;
        }
        match pos.tracker {
            Location::Indexed(iref) => {
                // FIXME: return Result<...>
                let line = self.source.read_line_at(iref.offset).unwrap();
                let eol = iref.offset + line.len();
                let range = if origin <= iref.offset {
                    // Moved forwards; range is [origin, end of line)
                    origin..eol
                } else {
                    // Moved backwards; range is [start of line, max(origin+1, end of line) )
                    iref.offset..(origin + 1).max(eol)
                };
                let line = LogLine::new(line, iref.offset);
                *pos = LogLocation { range, tracker: next_pos };
                Some(line)
            },
            _ => {
                pos.tracker = next_pos;
                None
            },
        }
    }

    fn next(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        // FIXME: Find a way to defer resolve_location until we're in read_line
        pos.tracker = self.resolve_location(pos.tracker);
        let next = self.index.next(pos.tracker);
        let ret = self.read_line(pos, next);
        ret
    }

    fn next_back(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        pos.tracker = self.resolve_location(pos.tracker);
        let next = self.index.next(pos.tracker);
        self.read_line(pos, next)
    }

    #[inline]
    fn len(&self) -> usize {
        self.source.len()
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
