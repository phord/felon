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
    pub range: LogRange,
    pub tracker: Option<Location>,
}

pub trait IndexedLog {
    /// Generate a cursor to use for reading lines from the file
    fn seek(&self, pos: usize) -> LogLocation {
        LogLocation {
            range: pos..pos,
            tracker: None,
        }
    }

    /// Read the next line from the file
    /// returns search results and the new cursor
    /// If line is None and pos.tracker is Some(Invalid), we're at the start of the file
    /// If line is None and tracker is anything else, there may be more to read
    fn next(&mut self, pos: &LogLocation) -> (Option<LogLine>, LogLocation);

    /// Read the previous line from the file
    /// returns search results and the new cursor
    /// If line is None and pos.tracker is Some(Invalid), we're at the start of the file
    /// If line is None and tracker is anything else, there may be more to read
    fn next_back(&mut self, pos: &LogLocation) -> (Option<LogLine>, LogLocation);


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

    // FIXME: remove this
    fn resolve_location(&mut self, pos: Location) -> Location;
}

pub trait IndexedLogOld {

    fn read_line_at(&mut self, start: usize) -> std::io::Result<String>;

    fn next_line_index(&self, find: Location) -> Location;

    fn prev_line_index(&self, find: Location) -> Location;
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

    fn next(&mut self, pos: &LogLocation) -> (Option<LogLine>, LogLocation) {
        // next line is after the range of the current one
        let start = pos.range.end;
        let find = {
            if let Some(p) = pos.tracker {
                // assert!(p.is_gap() || p.is_indexed());
                p
            } else {
                Location::Virtual(VirtualLocation::AtOrAfter(start))
            }
        };
        let find = self.resolve_location(find);
        let next = self.index.next_line_index(find);

        match next {
            Location::Indexed(pos) => {
                let line = self.source.read_line_at(pos.offset).unwrap(); // FIXME: return Result<...>
                let len = line.len();
                // let line = line.trim_end_matches('\n').to_string();
                let line = LogLine::new(line, pos.offset);
                let loc = LogLocation {
                    range: start..pos.offset + len,
                    tracker: Some(next),
                };
                (Some(line), loc)
            },

            _ => {
                let loc = LogLocation {
                    range: pos.range.clone(),
                    tracker: Some(next),
                };
                (None, loc)
            },
        }
    }

    fn next_back(&mut self, pos: &LogLocation) -> (Option<LogLine>, LogLocation) {
        // next line is after the range of the current one
        let end = pos.range.start;
        let find = {
            if let Some(p) = pos.tracker {
                // assert!(p.is_gap() || p.is_indexed());
                p
            } else {
                Location::Virtual(VirtualLocation::Before(end))
            }
        };
        let find = self.resolve_location(find);
        let next = self.index.prev_line_index(find);

        // FIXME: dedup with next
        match next {
            Location::Indexed(pos) => {
                let line = self.source.read_line_at(pos.offset).unwrap(); // FIXME: return Result<...>
                let len = line.len();
                // let line = line.trim_end_matches('\n').to_string();
                let line = LogLine::new(line, pos.offset);
                let loc = LogLocation {
                    range: pos.offset..end.max(pos.offset + len),
                    tracker: Some(next),
                };
                (Some(line), loc)
            },

            _ => {
                let loc = LogLocation {
                    range: pos.range.clone(),
                    tracker: Some(next),
                };
                (None, loc)
            },
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.source.len()
    }

    fn count_lines(&self) -> usize {
        todo!("self.index.count_lines()");
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
}

impl<LOG: LogFile> IndexedLogOld for LineIndexer<LOG> {
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
