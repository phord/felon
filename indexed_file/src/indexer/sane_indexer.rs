// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::{Seek, SeekFrom};
use crate::files::LogFile;
use crate::LogLine;

use super::indexed_log::IndexedLog;
use super::sane_index::SaneIndex;
use super::waypoint::{Position, VirtualPosition, Waypoint};


pub struct SaneIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: SaneIndex,
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
        }
    }
}

impl<LOG: LogFile> SaneIndexer<LOG> {
    #[inline]
    pub fn wait_for_end(&mut self) {
        self.source.wait_for_end()
    }

    fn next_line(&mut self, offset: usize) -> Option<LogLine> {
        if offset >= self.len() {
            return None;
        }
        let (bytes, line) = self.read_line(offset);
        line
    }

    fn prev_line(&mut self, offset: usize) -> Option<LogLine> {
        let (_bytes, line) = self.read_line(offset);
        line
    }

}

impl<LOG: LogFile> IndexedLog for SaneIndexer<LOG> {

    fn resolve_gap(&mut self, gap: std::ops::Range<usize>) -> std::io::Result<usize> {
        // Parse part or all of the gap and add it to our mapped index.
        self.source.seek(std::io::SeekFrom::Start(gap.start as u64))?;
        self.index.parse_bufread(&mut self.source, &gap)
    }

    fn read_line(&mut self, offset: usize) -> (usize, Option<LogLine>) {
        // FIXME: Use read_line_at;  it needs to return bytes read for us, though.
        // let line = self.source.read_line_at(offset).unwrap();
        let mut line = String::new();
        self.source.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
        // FIXME: make this safe for non-utf-8 sequences?
        let bytes = self.source.read_line(&mut line).unwrap();
        let logline = if bytes >  0 {
            Some(LogLine::new(line, offset))
        } else {
            None
        };
        (bytes, logline)
    }

    fn next(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        let original = pos.clone();
        let mut pos = pos;
        for _ in 0..5 {
            pos.clip(self.len());
            // Resolve position to next waypoint
            match pos.next(&self.index) {
                None => return (pos, None),

                Some(Waypoint::Mapped(offset)) => {
                    let next = self.next_line(offset);
                    if let Some(logline) = next {
                        #[allow(clippy::single_match)]
                        match original {
                            Position::Virtual(VirtualPosition::Offset(target)) => {
                                if (logline.offset..logline.offset + logline.line.len()).contains(&target) {
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

                Some(Waypoint::Unmapped(range)) => {
                    let start = range.start;
                    let chunk_size = 1024*1024;
                    let end = range.end.max(self.len()).min(start + chunk_size);
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

    fn next_back(&mut self, pos: Position) -> (Position, Option<LogLine>) {
        let original = pos.clone();
        let mut pos = pos;
        for _ in 0..5 {
            pos.clip(self.len());
            // Resolve position to prev waypoint
            match pos.next_back(&self.index) {
                None => return (pos, None),

                Some(Waypoint::Mapped(offset)) => {
                    let next = self.prev_line(offset);
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

                Some(Waypoint::Unmapped(range)) => {
                    let end = range.end.min(self.len());
                    let chunk_size = 1024*1024;
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
        let mut end = 0usize;
        self.index.iter()
            .filter(|w| matches!(w, Waypoint::Unmapped(_)))
            .fold(0usize, |acc, w| {
                if let Waypoint::Unmapped(range) = w {
                    let prev = end;
                    end = range.end;
                    acc + range.start - prev
                } else { unreachable!()}
            })
    }

    fn count_lines(&self) -> usize {
        self.index.iter().filter(|w| matches!(w, Waypoint::Mapped(_))).count()
    }

}
