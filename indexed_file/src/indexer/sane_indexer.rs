// Wrapper to discover and iterate log lines from a LogFile while memoizing parsed line offsets

use std::fmt;
use std::io::{Seek, SeekFrom};
use crate::files::LogFile;
use crate::LogLine;

use super::indexed_log::IndexedLog;
use super::sane_index::SaneIndex;
use super::waypoint::Waypoint;


pub struct SaneIndexer<LOG> {
    // pub file_path: PathBuf,
    source: LOG,
    index: SaneIndex,

    // File position if it was set or implied.
    // If not set, it's zero for next, eof for next_back.
    pos: Option<usize>,
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
            pos: None,
        }
    }

    fn get_pos(&self, def: usize) -> usize {
        self.pos.unwrap_or(def)
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
        self.pos = Some(offset + bytes);
        line
    }

    fn prev_line(&mut self, offset: usize) -> Option<LogLine> {
        let (_bytes, line) = self.read_line(offset);
        self.pos = Some(offset);
        line
    }

}

impl<LOG: LogFile> Seek for SaneIndexer<LOG> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                self.pos = Some(offset as usize);
                self.source.seek(SeekFrom::Start(offset))
            },
            SeekFrom::End(offset) => {
                self.pos = Some((self.source.len() as i64 - offset) as usize);
                self.source.seek(SeekFrom::End(offset))
            },
            SeekFrom::Current(offset) => {
                self.pos = Some((self.get_pos(0) as i64 + offset) as usize);
                self.source.seek(SeekFrom::Current(offset))
            },
        }
    }
}

impl<LOG: LogFile> IndexedLog for SaneIndexer<LOG> {

    fn seek(&mut self, pos: Option<usize>) {
        self.pos = pos;
    }

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

    fn next(&mut self) -> Option<LogLine> {
        for _ in 0..5 {
            let end = self.len();
            let start = self.get_pos(0).min(end);
            let start = self.index.find_at_or_after(start).waypoint?;
            let mut it = self.index.index.range(start..);
            match it.next() {
                Some(Waypoint::Mapped(offset)) => {
                    return self.next_line(*offset)
                },
                None => return None,
                Some(Waypoint::Unmapped(range)) => {
                    if range.start >= self.len() {
                        return None;
                    }
                    let start = range.start;
                    let chunk_size = 1024*1024;
                    let end = range.end.max(self.len()).min(start + chunk_size);
                    // FIXME: return errors
                    let _ = self.resolve_gap(start..end);
                },
            };
        }
        unreachable!("Failed to resolve gap 5 times?");
    }

    fn next_back(&mut self) -> Option<LogLine> {
        for _ in 0..5 {
            let end = self.len();
            let end = self.get_pos(end).min(end);
            let end = self.index.find_before(end).waypoint?;
            let mut it = self.index.index.range(..=end).rev();
            match it.next() {
                Some(Waypoint::Mapped(offset)) => {
                    return self.prev_line(*offset)
                },
                None => return None,
                Some(Waypoint::Unmapped(range)) => {
                    if range.start >= self.len() {
                        self.pos = Some(self.len() - 1);
                        continue;
                    }
                    let start = range.start.min(self.len());
                    let end = range.end.min(self.len());
                    let chunk_size = 1024*1024;
                    let start = start.max(end.saturating_sub(chunk_size));
                    // FIXME: return errors
                    let _ = self.resolve_gap(start..end);
                },
            }
        }
        unreachable!("Failed to resolve gap 5 times?");
    }

    #[inline]
    fn len(&self) -> usize {
        self.source.len()
    }

    fn indexed_bytes(&self) -> usize {
        let mut end = 0usize;
        self.index.index.iter()
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
        self.index.index.iter().filter(|w| matches!(w, Waypoint::Mapped(_))).count()
    }

}
