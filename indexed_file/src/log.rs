use crate::indexer::indexed_log::IndexStats;
use crate::indexer::sane_indexer::SaneIndexer;
use crate::indexer::waypoint::Position;
use crate::time_stamper::TimeStamper;
use crate::LogLine;
use std::path::PathBuf;
use crate::indexer::{GetLine, IndexedLog};

use crate::files::{new_text_file, LogBase, LogSource, Stream};

/**
 * Log is an adapter interface used to instantiate a SaneIndexer from different kinds of LogSources.
 */
pub struct Log {
    pub(crate) file: SaneIndexer<LogSource>,
    #[allow(dead_code)]
    pub(crate) format: TimeStamper,
    cached_len: usize,
}

impl<LOG: LogBase + 'static> From<LOG> for Log {
    fn from(file: LOG) -> Self {
        log::trace!("Instantiate log from LOG");
        let src = LogSource::from(file);
        Self::from(src)
    }
}

impl From<LogSource> for Log {
    fn from(src: LogSource) -> Self {
        log::trace!("Instantiate log via From<LogSource>");
        let src = SaneIndexer::new(src);
        let cached_len = src.len();
        Self {
            file: src,
            format: TimeStamper::default(),
            cached_len,
        }
    }
}

// Constructors
impl Log {
    pub fn new(src: SaneIndexer<LogSource>) -> Self {
        let cached_len = src.len();
        Self {
            file: src,
            format: TimeStamper::default(),
            cached_len,
        }
    }

    // unused?
    pub fn from_source(file: LogSource) -> Self {
        log::trace!("Instantiate log from LogSource");
        let src = SaneIndexer::new(file);
        let cached_len = src.len();
        Self {
            file: src,
            format: TimeStamper::default(),
            cached_len,
        }
    }

    pub fn open(file: Option<&PathBuf>) -> std::io::Result<Self> {
        log::trace!("Instantiate log from file {:?}", file);
        let src = new_text_file(file)?;
        let cached_len = src.len();
        let log = Log {
            file: SaneIndexer::new(src),
            format: TimeStamper::default(),
            cached_len,
        };
        Ok(log)
    }
}


// TODO: Delete this except for tests once SaneIndexer something something something...
impl Stream for Log {
    fn len(&self) -> usize {
        self.cached_len
    }

    // Wait on any data at all; Returns true if file is still open
    fn poll(&mut self) -> bool {
        self.file.poll()
    }

    fn is_open(&self) -> bool {
        self.file.is_open()
    }

    #[inline]
    fn wait_for_end(&mut self) {
        log::trace!("Wait for end of file");
        self.file.wait_for_end()
    }
}

// Navigation
// TODO: Delete this except for tests once SaneIndexer
impl IndexedLog for Log {

    #[inline]
    fn next(&mut self, pos: &Position) -> GetLine {
        self.file.next(pos)
    }

    #[inline]
    fn next_back(&mut self, pos: &Position) -> GetLine {
        self.file.next_back(pos)
    }

    fn advance(&mut self, pos: &Position) -> Position {
        self.file.advance(pos)
    }

    fn advance_back(&mut self, pos: &Position) -> Position {
        self.file.advance_back(pos)
    }

    fn info(&self) -> impl Iterator<Item = &IndexStats> + '_
    where Self: Sized
    {
        self.file.info()
    }

    #[inline]
    fn read_line(&mut self, offset: usize) -> Option<LogLine> {
        self.file.read_line(offset)
    }

    #[inline]
    fn set_timeout(&mut self, limit: Option<std::time::Duration>) {
        self.file.set_timeout(limit);
    }

    #[inline]
    fn timed_out(&mut self) -> bool {
        self.file.timed_out()
    }

    /// Determine if the current operation has timed out
    fn check_timeout(&mut self) -> bool {
        self.file.check_timeout()
    }

    fn resolve_gaps(&mut self, pos: &Position) -> Position {
        self.file.resolve_gaps(pos)
    }

    fn has_gaps(&self) -> bool {
        self.file.has_gaps()
    }
}
