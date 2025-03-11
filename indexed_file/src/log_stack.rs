use regex::Regex;

use crate::{files::Stream, index_filter::SearchType, indexer::{indexed_log::IndexStats, waypoint::Position, GetLine}, log_filter::LogFilter, IndexedLog, Log};

// Pending operations are used to handle blocking tasks asyncronously.  Mostly these are background tasks, and the order
// doesn't much matter.  But if we are searching for a line, we need to find it and return it asap.  So we try to complete
// searches first.  Then we try to fill gaps.  Then we try to stream new data. Streaming new data will cause us to have
// new gaps, so these last two might trade back and forth. But streaming arguably comes first because there are no gaps
// without data.

#[derive(Clone, Debug)]
enum PendingOp {
    //          count, offset
    SeekForward(usize, Position),
    SeekBackward(usize, Position),
    FillGaps(Position),
    Streaming,
    None,
}


impl PendingOp {
    fn seek_fwd_rev(&mut self, log: &mut LogFilter, src: &mut dyn IndexedLog, pos: Position) -> Position {
        match self {
            PendingOp::SeekForward(..) => log.find_next(src, &pos).into_pos(),
            PendingOp::SeekBackward(..) => log.find_next_back(src, &pos).into_pos(),
            _ => panic!("Invalid pending op: {:?} for {:?}", self, pos),
        }
    }

    fn update(&mut self, count: usize, pos: Position) -> Self {
        match self {
            PendingOp::SeekForward(..) => PendingOp::SeekForward(count, pos),
            PendingOp::SeekBackward(..) => PendingOp::SeekBackward(count, pos),
            _ => panic!("Invalid pending op: {:?} for {:?}", self, pos),
        }
    }
}


// TODO: Move this into Grok?  It implements some very lgt-specific features.

/// A stack of logs with filters.
/// Rust complicates our traits enough that it's impractical to rely on recursive log trees.
/// As it turns out, that's also impractical from a usability and reasoning standpoint, too.
/// This structure implements our complete stack of logs including the source files, include
/// filters, exclude filters, bookmarks, highlights and and searches.
pub struct LogStack {
    source: FilteredSource,
    search: Option<LogFilter>,  // FIXME: Should hold IndexFilter here and create a LogFilter as-needed
    pending: PendingOp,
}

impl  LogStack {
    pub fn new(log: Log) -> Self {
        Self {
            source: FilteredSource::new(log),
            search: None,
            pending: PendingOp::FillGaps(Position::invalid()),
        }
    }

    /// Apply a new regex search expression to the filter. If string is empty, clears current filter.
    /// TODO: add more filters instead of replacing the one we currently allow
    pub fn filter_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        self.source.filter_regex(re)?;
        // FIXME: Only restart search if filter changed
        if let Some(search) = &mut self.search {
            search.reset();
        }
        self.kick_pending();
        Ok(())
    }

    pub fn has_pending(&self) -> bool {
        !matches!(self.pending, PendingOp::None)
    }

    fn do_search(&mut self, timeout: u64, count: usize, pos: Position) -> Option<usize> {
        if let Some(ref mut search) = &mut self.search {
            let src = &mut self.source.with_timeout(timeout);
            let mut count = count;
            let mut pos = pos;
            loop {
                pos = self.pending.seek_fwd_rev(search, src, pos);
                if src.timed_out() || pos.is_invalid() {
                    break;
                }
                count = count.saturating_sub(1);
                if count == 0 {
                    break;
                }
            }
            if pos.is_invalid() {
                // Not found, but nothing more to find, either.
                self.pending = PendingOp::None;
                log::trace!("Search invalid");
                // FIXME: Return some indication to that caller that search failed
                None
            } else if src.timed_out() {
                // Didn't find it yet
                self.pending = self.pending.update(count, pos);
                log::trace!("Search timed out");
                None
            } else {
                // Found it
                log::trace!("Search found");
                self.pending = PendingOp::None;
                pos.offset()
            }
        } else {
            // No search term; nothing to do here
            self.pending = PendingOp::None;
            None
        }
    }

    fn do_fill_gaps(&mut self, timeout: u64, pos: Position) {
        let state= {
            let src = &mut self.with_timeout(timeout);
            let pos = src.resolve_gaps(&pos);
            if src.timed_out() {
                PendingOp::FillGaps(pos)
            } else {
                PendingOp::None
            }
        };
        self.pending = state;
    }

    pub fn describe_pending(&self) -> String {
        match self.pending {
            PendingOp::SeekForward(..) => "Search Forward".to_string(),
            PendingOp::SeekBackward(..) => "Search Backward".to_string(),
            PendingOp::FillGaps(..) => "Fill Gaps".to_string(),
            PendingOp::Streaming => "Streaming".to_string(),
            PendingOp::None => "None".to_string(),
        }
    }

    pub fn run_pending(&mut self, timeout: u64) -> Option<usize> {
        let mut result = None;
        match self.pending.clone() {
            PendingOp::SeekForward(count, pos) |
            PendingOp::SeekBackward(count, pos) =>
                result = self.do_search(timeout, count, pos),

            PendingOp::FillGaps(pos) =>
                self.do_fill_gaps(timeout, pos),

            PendingOp::Streaming => {
                let len = self.source.poll(Some(std::time::Instant::now() + std::time::Duration::from_millis(timeout)));
                if let Some(ref mut search) = &mut self.search {
                    search.update_len(len);
                }
                if !self.source.is_open() {
                    self.pending = PendingOp::None;
                }
            },

            PendingOp::None => {},
        };
        self.kick_pending();
        result
    }

    fn kick_pending(&mut self) {
        if matches!(self.pending, PendingOp::None) {
            if self.has_gaps() {
                self.pending = PendingOp::FillGaps(Position::invalid());
            } else if self.source.is_open() {
                self.pending = PendingOp::Streaming;
            }
        }
    }

    /// Set a new regex search expression
    /// TODO: allow multiple active searches
    pub fn search_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        if re.is_empty() {
            self.search = None;
        } else {
            self.search = Some(LogFilter::new(SearchType::Regex(Regex::new(re)?), self.source.len()));
            self.kick_pending();
        }
        Ok(())
    }

    pub fn search_next(&mut self, count: usize, offset: usize) -> Option<usize> {
        self.pending = PendingOp::SeekForward(count, Position::from(offset));
        // return a result if we have one within 10ms.  Otherwise, let caller run_pending on their own.
        self.run_pending(10)
    }

    pub fn search_next_back(&mut self, count: usize, offset: usize) -> Option<usize> {
        self.pending = PendingOp::SeekBackward(count, Position::from(offset));
        // return a result if we have one within 10ms.  Otherwise, let caller run_pending on their own.
        self.run_pending(10)
    }

}

impl Stream for LogStack {
    fn len(&self) -> usize {
        self.source.len()
    }

    // Poll for new data
    fn poll(&mut self, timeout: Option<std::time::Instant>) -> usize {
        self.source.poll(timeout)
    }
    fn is_open(&self) -> bool {
        self.source.is_open()
     }
}

impl IndexedLog for LogStack {
    fn read_line(&mut self, offset: usize) -> Option<crate::LogLine> {
        self.source.read_line(offset)
    }

    fn next(&mut self, pos: &Position) -> GetLine {
        self.source.next(pos)
    }

    fn next_back(&mut self, pos: &Position) -> GetLine {
        self.source.next_back(pos)
    }

    fn advance(&mut self, pos: &Position) -> Position {
        self.source.advance(pos)
    }

    fn advance_back(&mut self, pos: &Position) -> Position {
        self.source.advance_back(pos)
    }

    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        let pos = if pos.is_invalid() {
            self.seek(0)
        } else {
            pos.clone()
        };
        if let Some(ref mut search) = &mut self.search {
            if search.has_gaps() {
                return search.resolve_gaps(&mut self.source, &pos)
            }
        }

        self.source.resolve_gaps(&pos)
    }

    fn set_timeout(&mut self, limit: Option<std::time::Duration>) {
        self.source.set_timeout(limit);
    }

    fn timed_out(&mut self) -> bool {
        self.source.timed_out()
    }

    fn check_timeout(&mut self) -> bool {
        self.source.check_timeout()
    }

    fn info(&self) -> impl Iterator<Item = &'_ IndexStats> + '_
    where Self: Sized  {
        self.source.info()
        .chain(self.search.iter().flat_map(|f| f.info()))
    }

    fn has_gaps(&self) -> bool {
        self.source.has_gaps()
            || self.search.as_ref().map(|f| f.has_gaps()).unwrap_or(false)
    }
}

/// A wrapper layer to hold an optional filtered log.
/// This is primarily used to give us a detachable source so LogStack doesn't bump into Rust's ownership rules.
struct FilteredSource {
    source: Log,
    filter: Option<LogFilter>,
}

impl FilteredSource {
    pub fn new(source: Log) -> Self {
        Self { source, filter: None }
    }

    /// Apply a new regex search expression to the filter
    /// TODO: add more filters instead of replacing the one we currently allow
    pub fn filter_regex(&mut self, re: &str) -> Result<(), regex::Error> {
        if re.is_empty() {
            self.filter = None;
        } else {
            self.filter = Some(LogFilter::new(SearchType::new(re)?, self.source.len()));
        }
        Ok(())
    }
}

impl Stream for FilteredSource {
    fn len(&self) -> usize {
        self.source.len()
    }

    fn is_open(&self) -> bool {
        self.source.is_open()
    }

    // Poll for new data
    fn poll(&mut self, timeout: Option<std::time::Instant>) -> usize {
        let len = self.source.poll(timeout);
        if let Some(ref mut filter) = &mut self.filter {
            filter.update_len(len);
        }
        len
    }
}

impl IndexedLog for FilteredSource {
    fn read_line(&mut self, offset: usize) -> Option<crate::LogLine> {
        self.source.read_line(offset)
    }

    fn next(&mut self, pos: &Position) -> GetLine {
        if let Some(ref mut filter) = &mut self.filter {
            filter.find_next(&mut self.source, pos)
        } else {
            self.source.next(pos)
        }
    }

    fn next_back(&mut self, pos: &Position) -> GetLine {
        if let Some(ref mut filter) = &mut self.filter {
            filter.find_next_back(&mut self.source, pos)
        } else {
            self.source.next_back(pos)
        }
    }

    fn advance(&mut self, pos: &Position) -> Position {
        if let Some(ref mut filter) = &mut self.filter {
            filter.advance(pos)
        } else {
            self.source.advance(pos)
        }
    }

    fn advance_back(&mut self, pos: &Position) -> Position {
        if let Some(ref mut filter) = &mut self.filter {
            filter.advance_back(pos)
        } else {
            self.source.advance_back(pos)
        }
    }

    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        let pos = if pos.is_invalid() {
            self.seek(0)
        } else {
            pos.clone()
        };
        if let Some(ref mut filter) = &mut self.filter {
            if filter.has_gaps() {
                return filter.resolve_gaps(&mut self.source, &pos)
            }
        }

        if self.source.has_gaps() {
            return self.source.resolve_gaps(&pos)
        }

        Position::invalid()
    }

    fn set_timeout(&mut self, limit: Option<std::time::Duration>) {
        self.source.set_timeout(limit);
    }

    fn timed_out(&mut self) -> bool {
        self.source.timed_out()
    }

    fn check_timeout(&mut self) -> bool {
        self.source.check_timeout()
    }

    fn info(&self) -> impl Iterator<Item = &'_ IndexStats> + '_
    where Self: Sized  {
        self.source.info()
        .chain(self.filter.iter().flat_map(|f| f.info()))
    }

    fn has_gaps(&self) -> bool {
        self.source.has_gaps() ||
            self.filter.as_ref().map(|f| f.has_gaps()).unwrap_or(false)
    }
}