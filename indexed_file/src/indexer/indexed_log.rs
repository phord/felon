use std::time::Duration;
use crate::{LineIndexerIterator, LineViewMode, LogLine, SubLineIterator};

use super::{waypoint::Position, TimeoutWrapper};

// Result of fetching a line: got it, or timeout
pub enum GetLine {
    Hit(Position, LogLine),
    Miss(Position),
    Timeout(Position),
}

impl GetLine {
    pub fn into_pos(self) -> Position {
        match self {
            GetLine::Hit(pos, _) => pos,
            GetLine::Miss(pos) => pos,
            GetLine::Timeout(pos) => pos,
        }
    }
}

#[derive(Default, Debug)]
pub struct IndexStats {
    pub name: String,
    pub bytes_indexed: usize,
    pub lines_indexed: usize,
    pub bytes_total: usize,
}

impl IndexStats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Self::default()
        }
    }
}

pub trait IndexedLog {
    /// Return a Position to read from given offset.
    fn seek(&self, pos: usize) -> Position {
        Position::from(pos)
    }

    // Read the line at offset into a LogLine
    fn read_line(&mut self, offset: usize) -> Option<LogLine>;

    /// Read the next/prev line from the file
    /// returns search results and advances the file position
    /// If line is None, we're at the start/end of the file or we reached some limit (max time)
    /// Note: Unlike DoubleEndedIterator next_back, there is no rev() to reverse the iterator;
    ///    and "consumed" lines can still be read again.
    ///
    fn next(&mut self, pos: &Position) -> GetLine;
    fn next_back(&mut self, pos: &Position) -> GetLine;

    /// Resolve any gap in the index by reading the log from the source.
    /// Return Position where we stopped if we timed out
    fn resolve_gaps(&mut self, pos: &Position) -> Position;

    /// Set a time limit for operations that may take too long
    fn set_timeout(&mut self, _limit: Option<Duration>);

    /// Determine if previous operation exited due to timeout
    fn timed_out(&mut self) -> bool;

    /// Length of the log in total bytes
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterator to provide access to info about the different indexes
    fn info(&self) -> impl Iterator<Item = &'_ IndexStats> + '_
    where Self: Sized ;

    // Autowrap
    fn with_timeout(&mut self, ms: usize) -> TimeoutWrapper<Self> where Self: std::marker::Sized {
        TimeoutWrapper::new(self, ms)
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
    fn iter_lines_range<'a, R>(&'a mut self, range: &'a R) -> impl DoubleEndedIterator<Item = LogLine> + 'a
    where R: std::ops::RangeBounds<usize>,
        Self: Sized {
        self.iter_view_from(LineViewMode::WholeLine, range)
    }

    // TEST and MergedLog
    fn iter_view(&mut self, mode: LineViewMode) -> impl DoubleEndedIterator<Item = LogLine> + '_
    where Self: Sized {
        SubLineIterator::new(self, mode)
    }

    // Used in FilteredLog and Document (grok)
    fn iter_view_from<'a, R>(&'a mut self, mode: LineViewMode, range: &'a R) -> impl DoubleEndedIterator<Item = LogLine> + 'a
    where
        R: std::ops::RangeBounds<usize>,
        Self: Sized {
        SubLineIterator::range(self, mode, range)
    }
}