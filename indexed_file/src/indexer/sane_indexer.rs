// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::time::Duration;
use crate::files::LogFile;
use crate::LogLine;

use super::indexed_log::IndexedLog;
use super::sane_index::SaneIndex;
use super::timeout::Timeout;
use super::waypoint::{Position, VirtualPosition};
use super::GetLine;

struct IndexerStats {
    stale: bool,
    bytes_indexed: usize,
    lines_indexed: usize,
}

impl Default for IndexerStats {
    fn default() -> Self {
        Self {
            stale: true,
            bytes_indexed: 0,
            lines_indexed: 0,
        }
    }
}

pub struct SaneIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: SaneIndex,
    timeout: Timeout,

    stats: IndexerStats,
}

impl<LOG: LogFile> fmt::Debug for SaneIndexer<LOG> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaneIndexer")
         .finish()
    }
}

const CHUNK_SIZE:usize = 64 * 1024;

impl<LOG: LogFile> SaneIndexer<LOG> {

    pub fn new(file: LOG) -> SaneIndexer<LOG> {
        Self {
            source: file,
            index: SaneIndex::new(),
            timeout: Timeout::Inactive(false),
            stats: IndexerStats::default(),
        }
    }

    /// Read the line starting from offset to EOL
    fn read_line_from(&mut self, offset: usize) -> Option<LogLine> {
        // Find the line containing offset, if any
        let line = self.source.read_line_at(offset).unwrap();
        if !line.is_empty() {
            Some(LogLine::new(line, offset))
        } else {
            None
        }
    }

    /// read and memoize a line containing a given offset from a BufRead
    /// Returns the indexed position and the Logline, if found; else None
    /// FIXME: return errors
    fn read_line_memo(&mut self, pos: &Position, offset: usize) -> GetLine {
        if self.timeout.is_timed_out() {
            Err(())
        } else if offset >= self.len() {
            Ok((Position::Virtual(VirtualPosition::End), None))
        } else {
            let next = self.read_line_from(offset);

            let mut pos = pos.resolve(&self.index);
            if pos.is_unmapped() {
                if let Some(ref line) = next {
                    pos = self.index.insert_one(&pos, &(line.offset..line.offset + line.line.len()));
                } else {
                    panic!("Read error?");
                }
            }
            Ok((pos, next))
        }
    }

    /// Reads line indicated by pos and memoizes it.
    /// Returns the memoized position and the line read.
    pub fn read_line_at(&mut self, pos: &Position) -> GetLine {
        // Resolve position to a target offset to read in the file
        let offset = pos.least_offset();
        if offset >= self.len() {
            return Ok((Position::Virtual(VirtualPosition::End), None));
        }
        self.read_line_memo(pos, offset)
    }

    // Find the last line in the range [start, end], memoizing all complete lines we see.
    // Return last memoized line from the region.
    fn last_line(&mut self, start: usize, end: usize) -> GetLine {
        // We're scanning a chunk of memory that has 0 or more line-endings in it.
        //       start                  end
        //         v                     v
        // ----|---[===|============|====]-----|-----
        //                          ^last line^   This is the line we want
        //             ^another line^             This is a complete line, so we will memoize it.
        //      ^prev--^                          This line is only partially scanned, so we will not memoize it.
        //
        // Try reading the first (throwaway) line.
        //   If it ends before our endpoint, memoize the rest of the lines and return the last one.
        //   Otherwise, fail.
        let offset =
            if start == 0 {
                // If we start at zero, there are no throwaway lines
                0
            } else if let Some(line) = self.read_line_from(start) {
                // Found a partial line, but we only need to know where it ends to establish a foothold
                let offset = line.offset + line.line.len();
                if offset > end {
                    // Did not find a line break in our gap. Failure.
                    return Ok((Position::invalid(), None));
                }
                offset
            } else {
                // Did not find anything.  EOF?
                panic!("Reading past EOF intentionally?");
                return Ok((Position::invalid(), None));
            };

        // Found the start of a line in our gap. Read from here to end of gap and remember the lines.
        let mut pos = Position::Virtual(VirtualPosition::Offset(offset));
        loop {
            let (p, line) = self.read_line_at(&pos)?;
            if p.is_invalid() {
                unreachable!("What?");
                return Ok((p, None));
            }
            if p.most_offset() > end {
                return Ok((p, line));
            }
            pos = p.next(&self.index);
        }
    }

    /// Scan a chunk of space bounded by pos before the offset position to find the start of our target line
    /// Return the last line found before offset in the region.
    /// Note: offset is inclusive
    fn scan_lines_backwards(&mut self, pos: &Position, offset: usize) -> GetLine {
        assert!(pos.is_unmapped());

        // TODO: Get efficient chunk offsets from the underlying LOG type.
        let mut chunk_delta = CHUNK_SIZE;

        // Scan one byte before this region to ensure we can use the EOL from the previous matched chunk as a baseline
        let start = pos.least_offset().saturating_sub(1);
        loop {
            let try_offset = offset.saturating_sub(chunk_delta).max(start);
            let (pos, line) = self.last_line(try_offset, offset)?;
            if !pos.is_invalid() && pos.most_offset() >= offset {
                // Found the line touching our endpoint
                assert!(pos.least_offset() <= offset);
                return Ok((pos, line));
            }
            assert!(pos.least_offset() >= start);
            if pos.least_offset() == start {
                panic!("This doesn't happen, does it?");
                // return Ok((pos, line));
            }
            if try_offset == start {
                // Scanned whole gap but didn't find any new line breaks.  How did we get this gap?
                panic!("Inconsistent index?  Gap has no line breaks.");
            }
            if chunk_delta > offset {
                // Scanned whole gap but didn't find any new line breaks.  How did we get this gap?
                panic!("Inconsistent index?  Gap has no line breaks.");
            }
            // No lines found.  Scan a larger chunk.
            chunk_delta *= 2;
        }
    }
}

impl<LOG: LogFile> SaneIndexer<LOG> {
    #[inline]
    pub fn wait_for_end(&mut self) {
        self.source.wait_for_end()
    }

    fn get_line(&mut self, offset: usize) -> Option<LogLine> {
        if offset >= self.len() {
            return None;
        }
        self.read_line(offset)
    }
}

impl<LOG: LogFile> IndexedLog for SaneIndexer<LOG> {

    fn set_timeout(&mut self, limit: Option<Duration>) {
        self.timeout.set(limit);
    }

    // reports if the current timeout
    fn timed_out(&mut self) -> bool {
        self.timeout.timed_out() || self.timeout.prev_timed_out()
    }

    fn read_line(&mut self, offset: usize) -> Option<LogLine> {
        // TODO: return errors?
        let line = self.source.read_line_at(offset).unwrap();
        if !line.is_empty() {
            Some(LogLine::new(line, offset))
        } else {
            None
        }
    }

    fn next(&mut self, pos: &Position) -> GetLine {
        self.timeout.active();
        let offset = pos.least_offset().min(self.len());
        let pos = pos.resolve(&self.index);
        Ok(if offset >= self.len() {
            (Position::invalid(), None)
        } else if pos.is_mapped() || offset == pos.least_offset() {
            let (pos, line) = self.read_line_at(&pos)?;
            (pos.next(&self.index), line)
        } else if pos.is_unmapped() {
            // Unusual case: We're reading from some offset in the middle of a gap.  Scan backwards to find the start of the line.
            let (pos, line) = self.scan_lines_backwards(&pos, offset)?;
            (pos.next(&self.index), line)
        } else {
            // Does this happen?
            (pos, None)
        })
    }

    fn next_back(&mut self, pos: &Position) -> GetLine {
        self.timeout.active();
        let offset = pos.most_offset().min(self.len());
        if offset == 0 {
            return Ok((Position::invalid(), None));
        }
        let mut pos = pos.resolve_back(&self.index);
        if pos.least_offset() >= self.len() {
            pos = pos.advance_back(&self.index);
            assert!(pos.least_offset() < self.len())
        }

        let (pos, line) =
        if pos.is_invalid() {
            (pos, None)
        } else if pos.is_mapped() {
            self.read_line_at(&pos)?
        } else {
            // Scan backwards, exclusive of end pos
            self.scan_lines_backwards(&pos, offset - 1)?
        };
        Ok((pos.next_back(&self.index), line))
    }

    #[inline]
    fn len(&self) -> usize {
        self.source.len()
    }

    #[inline]
    fn indexed_bytes(&self) -> usize {
        self.index.indexed_bytes()
    }

    #[inline]
    fn count_lines(&self) -> usize {
        self.index.count_lines()
    }

}
