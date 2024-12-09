// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::{Seek, SeekFrom};
use crate::files::LogFile;
use crate::LogLine;

use super::indexed_log::IndexedLog;
use super::sane_index::SaneIndex;
use super::waypoint::{Position, VirtualPosition, Waypoint};

struct indexer_stats {
    stale: bool,
    bytes_indexed: usize,
    lines_indexed: usize,
}

impl Default for indexer_stats {
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

    stats: indexer_stats,
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
            stats: indexer_stats::default(),
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

    fn resolve_gap(&mut self, gap: std::ops::Range<usize>) -> std::io::Result<usize> {
        // Parse part or all of the gap and add it to our mapped index.
        self.source.seek(std::io::SeekFrom::Start(gap.start as u64))?;
        let ret = self.index.parse_bufread(&mut self.source, &gap);
        ret
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

    fn next(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        let original = pos.clone();
        let mut pos = pos;
        for _ in 0..5 {
            // Resolve position to next waypoint
            match pos.next(&self.index) {
                None => return (pos, None),

                Some((Waypoint::Mapped(range), _target)) => {
                    let next = self.get_line(range.start);
                    if let Some(logline) = next {
                        #[allow(clippy::single_match)]
                        match original {
                            Position::Virtual(VirtualPosition::Offset(target)) => {
                                if (logline.offset..logline.offset + logline.line.len()).contains(&target) {
                                    return (pos, Some(logline));
                                }
                                // else -- next loop will find the correct line with pos.next(), or unmapped, and we'll do this dance again.
                                return (pos, Some(logline));
                            },
                            _ => {
                                return (pos, Some(logline));
                            },
                        }
                    } else {
                        return (pos, None)
                    }
                },

                Some((Waypoint::Unmapped(range), target)) => {
                    assert!(range.contains(&target));

                    let chunk_size = 1024*1024;
                    let start = target.saturating_sub(1).max(range.start);
                    let end = range.end.min(self.len()).min(start + chunk_size);
                    let start = end.saturating_sub(chunk_size).min(start).max(range.start);
                    if start >= end {
                        return (pos, None);
                    }
                    // FIXME: return errors
                    match self.resolve_gap(start..end) {
                        Ok(0) => return (pos, None),
                        Err(_) => return (pos, None), // TODO Pass errors upstream
                        _ => {},
                    }
                    pos = original.clone();
                },
            };
        }
        unreachable!("Failed to resolve gap 5 times?");
    }

    // this is a near-dup of next, but it has several subtle differences.  :-(
    fn next_back(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        let original = pos.clone();
        let mut pos = pos;
        for _ in 0..5 {
            pos.clip(self.len());
            // Resolve position to prev waypoint
            match pos.next_back(&self.index) {
                None => return (pos, None),

                Some((Waypoint::Mapped(range), _target)) => {
                    let next = self.get_line(range.start);
                    if let Some(logline) = next {
                        #[allow(clippy::single_match)]
                        match original {
                            Position::Virtual(VirtualPosition::Offset(target)) => {
                                if logline.offset < target {
                                    return (pos, Some(logline));
                                }
                                // else -- next loop will find the correct line with pos.next(), or unmapped, and we'll do this dance again.
                            },
                            _ => {
                                return (pos, Some(logline));
                            },
                        }
                    } else {
                        return (pos, None)
                    }
                },

                Some((Waypoint::Unmapped(range), target)) => {
                    let chunk_size = 1024*1024;
                    let end = target.saturating_add(chunk_size).min(range.end).min(self.len());
                    let start = end.saturating_sub(chunk_size).max(range.start);
                    if start >= end {
                        return (pos, None);
                    }
                    // FIXME: return errors
                    let _ = self.resolve_gap(start..end);
                    // FIXME: Adjust pos index by adding inserted waypoints
                    pos = original.clone();
                },
            };
        }
        unreachable!("Failed to resolve gap 5 times?");
    }

    #[inline]
    fn len(&self) -> usize {
        self.source.len()
    }

    fn indexed_bytes(&self) -> usize {
        self.stats.bytes_indexed
    }

    fn count_lines(&self) -> usize {
        self.stats.lines_indexed
    }

}
