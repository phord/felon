use std::ops::Range;
use std::time::{Duration, Instant};
use crate::{LineIndexerIterator, LineViewMode, LogLine, SubLineIterator};
use super::eventual_index::{Location, VirtualLocation};



type LogRange = Range<usize>;

#[derive(Debug)]
pub struct LogLocation {
    // The range we scanned to return the current line
    pub range: LogRange,

    // The location of the next line to read
    pub tracker: Location,

    /// The time we want to allow to find the next line
    pub timeout: Option<Instant>,
}

impl LogLocation {
    pub fn set_timeout(self, ms: u64) -> LogLocation{
        let timeout = Duration::from_millis(ms);
        LogLocation {
            timeout: Some(Instant::now() + timeout),
            ..self
        }
    }

    pub fn elapsed(&self) -> bool {
        match self.timeout {
            Some(timeout) => Instant::now() > timeout,
            None => false,
        }
    }
}

pub enum LineOption {
    /// We found a logline
    Line(LogLine),

    /// We timed out looking for the next line
    Checkpoint,

    /// We reached the end of the file
    None,
}

impl LineOption {
    pub fn is_some(&self) -> bool {
        matches!(self, LineOption::Line(_))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, LineOption::None)
    }

    pub fn is_checkpoint(&self) -> bool {
        matches!(self, LineOption::Checkpoint)
    }

    pub fn unwrap(self) -> LogLine {
        match self {
            LineOption::Line(line) => line,
            _ => panic!("Called unwrap on LineOption::None"),
        }
    }
}

pub trait IndexedLog {
    /// Generate a cursor to use for reading lines from the file
    fn seek(&self, pos: usize) -> LogLocation {
        LogLocation {
            range: pos..pos,
            tracker: Location::Virtual(VirtualLocation::AtOrAfter(pos)),
            timeout: None,
        }
    }

    /// Generate a cursor to use for reading lines from the file
    fn seek_rev(&self, pos: usize) -> LogLocation {
        LogLocation {
            range: pos..pos,
            tracker: Location::Virtual(VirtualLocation::Before(pos)),
            timeout: None,
        }
    }

    fn find_gap(&mut self) -> LogLocation ;

    // Read the line at pos, if any, and return the iterator results and the new cursor
    fn read_line(&mut self, pos: &mut LogLocation, next_pos: Location) -> Option<LogLine>;

    /// Read the next line from the file
    /// returns search results and modifies the cursor with updated info
    /// If line is None and pos.tracker is Invalid, we're at the start/end of the file
    /// If line is None and tracker is anything else, there may be more to read
    fn next(&mut self, pos: &mut LogLocation) -> LineOption;

    fn iter_next(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        for i in 0..5 {
            // We should have resolved it by now
            assert!(i<4);
            let line = self.next(pos);
            if line.is_some() {
                return Some(line.unwrap())
            } else if pos.tracker.is_invalid() {
                return None
            }
        }
        // unreachable!("We failed to read a line in 5 tries");
        None
    }

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

    fn indexed_bytes(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn count_lines(&self) -> usize ;

}