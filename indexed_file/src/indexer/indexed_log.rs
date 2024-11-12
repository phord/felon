use std::ops::Range;
use std::time::{Duration, Instant};
use crate::{LineIndexerIterator, LineViewMode, LogLine, SubLineIterator};

pub trait IndexedLog {
    fn resolve_gap(&mut self, gap: std::ops::Range<usize>) -> std::io::Result<usize> {
        todo!("resolve_gap");
    }

    /// Position log to read from given offset
    fn seek(&mut self, pos: Option<usize>);

    // Read the line at pos, if any, and return the iterator results and the new cursor
    fn read_line(&mut self, offset: usize) -> (usize, Option<LogLine>);

    /// Read the next line from the file
    /// returns search results and advances the file position
    /// If line is None, we're at the start/end of the file
    /// If line is CheckPoint, there may be more to read
    fn next(&mut self) -> Option<LogLine>;
    fn next_back(&mut self) -> Option<LogLine>;

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