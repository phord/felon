use std::ops::Range;
use std::time::{Duration, Instant};
use crate::{LineIndexerIterator, LineViewMode, LogLine, SubLineIterator};

pub trait IndexedLog {
    /// Position log to read from given offset, or None for start/end of file.
    /// If None, the cursor is set to the start of the file if next() is called,
    /// or to the end of the file when next_back() is called. Once either is called,
    /// the cursor will remain on the last read line.  This a convenience to permit
    /// iterators to be built that can be use simply for file navigation.
    fn seek(&mut self, pos: Option<usize>);

    // Read the line at offset, if any, and return the iterator result and the number of bytes consumed.
    // Note the length of the line may be modified to fit utf-8 charset, so the bytes consumed may be
    // different than the string length. The new file position will be the offset + the bytes consumed.
    // FIXME: We should return the new offset, not the bytes consumed.
    fn read_line(&mut self, offset: usize) -> (usize, Option<LogLine>);

    /// Read the next/prev line from the file
    /// returns search results and advances the file position
    /// If line is None, we're at the start/end of the file or we reached some limit (max time)
    /// Note: these form a non-conforming iterator since these functions do not consume an iterable range.
    ///       For example,
    ///         log.seek(Some(offset));
    ///         let a = log.next();
    ///         let b = log.next();
    ///         let c = log.next_back();
    ///         let d = log.next_back();
    ///         assert!(b == c);
    ///         assert!(a == d);
    ///
    fn next(&mut self) -> Option<LogLine>;
    fn next_back(&mut self) -> Option<LogLine>;

    /// Resolve the gap in the index by reading the log from the source.
    /// Returns number of bytes indexed during this operation.
    /// FIXME: Make gap an Option<>, where None means to find any remaining gaps and work on them.
    fn resolve_gap(&mut self, gap: std::ops::Range<usize>) -> std::io::Result<usize> {
        todo!("resolve_gap");
    }

    /// Set a time limit for operations that may take too long
    fn set_timeout(&mut self, limit: Option<Duration>) -> Instant {
       todo!("force_time_limit");
    }

    /// Length of the log in total bytes
    fn len(&self) -> usize;

    /// Actual indexed bytes in the log; gives an indication of the completeness of the index
    fn indexed_bytes(&self) -> usize;

    /// Count of known lines in the log (may be undercounted if index is incomplete)
    fn count_lines(&self) -> usize ;

    fn is_empty(&self) -> bool {
        self.len() == 0
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
}