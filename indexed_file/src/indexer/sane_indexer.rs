// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::{Seek, SeekFrom};
use crate::files::LogFile;
use crate::indexer::eventual_index::{Location, GapRange, Missing::{Bounded, Unbounded}};
use crate::LogLine;

use super::indexed_log::{IndexedLog, LineOption, LogLocation};
use super::sane_index::SaneIndex;


pub struct SaneIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: SaneIndex,
    pos: u64,
}

impl<LOG: LogFile> fmt::Debug for SaneIndexer<LOG> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaneIndexer")
         .finish()
    }
}

impl<LOG> SaneIndexer<LOG> {

    pub fn new(file: LOG) -> SaneIndexer<LOG> {
        Self {
            source: file,
            index: SaneIndex::new(),
            pos: 0,
        }
    }
}

impl<LOG: LogFile> SaneIndexer<LOG> {
    #[inline]
    pub fn wait_for_end(&mut self) {
        self.source.wait_for_end()
    }

    // fill in any gaps by parsing data from the file when needed
    #[inline]
    fn resolve_location(&mut self, pos: &mut LogLocation) {
        // Resolve any virtuals into gaps or indexed
        // pos.tracker = self.index.resolve(pos.tracker, self.len());

        // Resolve gaps
        for _ in 0..10 {
            if !pos.tracker.is_gap() || pos.elapsed() { return; }
            pos.tracker = self.index_chunk(pos.tracker);
        }
        assert!(!pos.tracker.is_gap());
    }
}

impl<LOG: LogFile> Seek for SaneIndexer<LOG> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        todo!("self.index.seek(pos)");
    }
}

impl<LOG: LogFile> IndexedLog for SaneIndexer<LOG> {


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
                pos.range = range;
                pos.tracker = next_pos;
                Some(line)
            },
            _ => {
                pos.tracker = next_pos;
                None
            },
        }
    }

    fn next(&mut self, pos: &mut LogLocation) -> LineOption {
        self.resolve_location(pos);
        if pos.elapsed() {
            LineOption::Checkpoint
        } else {
            // let next = self.index.next(pos.tracker);
            // if let Some(line) = self.read_line(pos, next) {
            //     LineOption::Line(line)
            // } else {
                LineOption::None
            // }
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.source.len()
    }

    fn indexed_bytes(&self) -> usize {
        todo!("self.index.indexed_bytes()");
    }

    fn count_lines(&self) -> usize {
        todo!("self.index.count_lines()");
    }

    fn find_gap(&mut self) -> LogLocation  {
        todo!()
    }

}


impl<LOG: LogFile> SaneIndexer<LOG> {

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
            self.index.parse_bufread(&mut self.source, &(start..end)).expect("Ignore read errors");
            Location::Invalid // FIXME
        }
    }


    pub fn count_lines(&self) -> usize {
        todo!("self.index.lines()")
    }
}
