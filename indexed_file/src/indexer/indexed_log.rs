use std::time::Duration;
use crate::{files::Stream, LineIndexerDataIterator, LineIndexerIterator, LogLine};

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
    pub fn new(name: String, bytes_total: usize) -> Self {
        Self {
            name,
            bytes_total,
            ..Self::default()
        }
    }

    pub fn reset(&mut self) {
        self.bytes_indexed = 0;
        self.lines_indexed = 0;
    }
}

pub trait IndexedLog: Stream {
    /// Return a Position to read from given offset.
    /// Always returns a generic virtual position that can be used on any index.
    fn seek(&self, pos: usize) -> Position {
        Position::from(pos)
    }

    // Read the line at offset into a LogLine
    // TODO: Move this into Log
    fn read_line(&mut self, offset: usize) -> Option<LogLine>;

    /// Read the next/prev line from the file
    /// returns
    ///    GetLine::Hit:     found line and it's indexed position
    ///    GetLine::Miss:    not found
    ///    GetLine::Timeout: we reached some limit (max time); pos is where we stopped
    /// Note: Unlike DoubleEndedIterator next_back, there is no rev() to reverse the iterator;
    ///    and "consumed" lines can still be read again.
    fn next(&mut self, pos: &Position) -> GetLine;
    fn next_back(&mut self, pos: &Position) -> GetLine;

    /// Advance the position to the next/prev waypoint
    fn advance(&mut self, pos: &Position) -> Position;
    fn advance_back(&mut self, pos: &Position) -> Position;

    /// Resolve any gap in the index by reading the log from the source.
    /// Return Position where we stopped if we timed out, or Invalid if we're fully indexed
    fn resolve_gaps(&mut self, pos: &Position) -> Position;

    /// Returns true if there are any gaps in the index
    fn has_gaps(&self) -> bool;

    /// Set a time limit for operations that may take too long
    fn set_timeout(&mut self, _limit: Option<Duration>);

    /// Determine if previous operation exited due to timeout
    fn timed_out(&mut self) -> bool;

    /// Determine if the current operation has timed out
    fn check_timeout(&mut self) -> bool;

    /// Iterator to provide access to info about the different indexes
    fn info(&self) -> impl Iterator<Item = &'_ IndexStats> + '_
    where Self: Sized ;

    // Autowrap
    fn with_timeout(&mut self, ms: u64) -> TimeoutWrapper<Self> where Self: std::marker::Sized {
        TimeoutWrapper::new(self, ms as usize)
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
        LineIndexerDataIterator::new(self)
    }

    // Used in tests and in bin/tail
    fn iter_lines_range<'a, R>(&'a mut self, range: &'a R) -> impl DoubleEndedIterator<Item = LogLine> + 'a
    where R: std::ops::RangeBounds<usize>,
        Self: Sized {
        LineIndexerDataIterator::range(self, range)
    }
}