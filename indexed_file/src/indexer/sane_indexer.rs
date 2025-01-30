// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

extern crate lru;

use std::fmt;
use std::time::Duration;
use std::num::NonZeroUsize;
use lru::LruCache;

use crate::files::LogFile;
use crate::LogLine;

use super::indexed_log::{IndexStats, IndexedLog};
use super::sane_index::SaneIndex;
use super::timeout::Timeout;
use super::waypoint::{Position, VirtualPosition};
use super::GetLine;

pub struct SaneIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: SaneIndex,
    timeout: Timeout,
    line_cache: LruCache<usize, LogLine>,
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
        let len = file.len();
        // FIXME: Pass filename instead of generic token
        let index = SaneIndex::new("File".to_string(), len);
        Self {
            source: file,
            index,
            timeout: Timeout::Inactive(false),
            line_cache: LruCache::new(NonZeroUsize::new(1000).unwrap()),
        }
    }

    /// read and memoize a line containing a given offset from a BufRead
    /// Returns Hit(found_line), Miss(EOF), or Timeout(pos)
    /// FIXME: return errors from read_line
    fn get_line_memo(&mut self, pos: &Position) -> GetLine {
        // Resolve position to a target offset to read in the file
        let offset = pos.least_offset();
        if self.check_timeout() {
            GetLine::Timeout(pos.clone())
        } else if offset >= self.len() {
            GetLine::Miss(Position::invalid())
        } else {
            let next = self.read_line(offset);

            let mut pos = pos.resolve(&self.index);
            if pos.is_unmapped() {
                if let Some(ref line) = next {
                    pos = self.index.insert_one(&pos, &(line.offset..line.offset + line.line.len()));
                } else {
                    panic!("Read error? offset={}", offset);
                }
            }
            GetLine::Hit(pos, next.unwrap_or_default())
        }
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
            } else if let Some(line) = self.read_line(start) {
                // Found a partial line, but we only need to know where it ends to establish a foothold
                let offset = line.offset + line.line.len();
                if offset > end {
                    // Did not find a line break in our gap. Failure.
                    return GetLine::Miss(Position::invalid());
                }
                offset
            } else {
                // Did not find anything.  EOF?
                panic!("Reading past EOF intentionally?");
                // return GetLine::Miss(Position::invalid());
            };

        // Found the start of a line in our gap. Read from here to end of gap and remember the lines.
        let mut pos = Position::Virtual(VirtualPosition::Offset(offset));
        loop {
            let get = self.get_line_memo(&pos);
            if let GetLine::Hit(p, _) = &get {
                if p.most_offset() > end {
                    return get;
                }
                pos = p.next(&self.index);
            } else {
                // No lines matched? Failure.
                return get;
            }
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
            let get = self.last_line(try_offset, offset);
            if let GetLine::Hit(pos, _) = &get {
                if !pos.is_invalid() && pos.most_offset() >= offset {
                    // Found the line touching our endpoint
                    assert!(pos.least_offset() <= offset);
                    return get;
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
            } else {
                return get;
            }
        }
    }
}

impl<LOG: LogFile> SaneIndexer<LOG> {
    #[inline]
    pub fn wait_for_end(&mut self) {
        self.source.wait_for_end()
    }
}

impl<LOG: LogFile> IndexedLog for SaneIndexer<LOG> {

    fn set_timeout(&mut self, limit: Option<Duration>) {
        self.timeout.set(limit);
    }

    // reports if the current operation is timed out
    fn timed_out(&mut self) -> bool {
        self.timeout.timed_out() || self.timeout.prev_timed_out()
    }

    // check for timeout and latch the result
    // Returns true one time, when the timeout is first detected.  Thereafter returns false.
    fn check_timeout(&mut self) -> bool {
        self.timeout.is_timed_out()
    }

    /// Read the line starting from offset to EOL
    fn read_line(&mut self, offset: usize) -> Option<LogLine> {
        // Find the line containing offset, if any
        let line = self.line_cache.get(&offset);
        if let Some(line) = line {
            return Some(line.clone());
        }
        let line = self.source.read_line_at(offset).unwrap();
        if !line.is_empty() {
            let line = LogLine::new(line, offset);
            self.line_cache.put(offset, line.clone());
            Some(line)
        } else {
            None
        }
    }

    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        let mut pos = self.index.seek_gap(pos);
        while pos.is_unmapped() {
            // Resolve unmapped region
            match self.get_line_memo(&pos) {
                GetLine::Hit(p, _) =>     // Found a line.  Advance to the remaining gap, if any.
                        { assert!(p.is_mapped()); pos = p.advance(&self.index)},
                GetLine::Miss(p) =>       // End of file
                        return p,
                GetLine::Timeout(p) =>    // Timeout.  Return the position we stopped at.
                        return p,
            }
        }
        Position::invalid()
    }

    fn next(&mut self, pos: &Position) -> GetLine {
        self.timeout.active();
        let offset = pos.least_offset().min(self.len());
        let pos = pos.resolve(&self.index);
        if offset >= self.len() {
            GetLine::Miss(Position::invalid())
        } else if pos.is_mapped() || offset == pos.least_offset() {
            self.get_line_memo(&pos)
        } else if pos.is_unmapped() {
            // Unusual case: We're reading from some offset in the middle or start of a gap.  Scan backwards to find the start of the line.
            self.scan_lines_backwards(&pos, offset)
        } else {
            // Does this happen?
            GetLine::Miss(pos)
        }
    }

    fn next_back(&mut self, pos: &Position) -> GetLine {
        self.timeout.active();
        let offset = pos.most_offset().min(self.len());
        if offset == 0 {
            return GetLine::Miss(Position::invalid());
        }
        let mut pos = pos.resolve_back(&self.index);
        if pos.least_offset() >= self.len() {
            pos = pos.advance_back(&self.index);
            assert!(pos.least_offset() < self.len())
        }

        if pos.is_invalid() {
            GetLine::Miss(pos)
        } else if pos.is_mapped() {
            self.get_line_memo(&pos)
        } else {
            // Scan backwards, exclusive of end pos
            self.scan_lines_backwards(&pos, offset - 1)
        }
    }

    fn advance(&mut self, pos: &Position) -> Position {
        pos.next(&self.index)
    }

    fn advance_back(&mut self, pos: &Position) -> Position {
        pos.next_back(&self.index)
    }


    #[inline]
    fn len(&self) -> usize {
        self.index.stats.bytes_total
    }

    fn info(&self) -> impl Iterator<Item = &IndexStats> + '_
    where Self: Sized
    {
        std::iter::once(&self.index.stats)
    }

    fn has_gaps(&self) -> bool {
        self.index.stats.bytes_indexed < self.index.stats.bytes_total
    }

}
