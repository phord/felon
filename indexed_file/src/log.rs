use crate::indexer::sane_indexer::SaneIndexer;
use crate::time_stamper::TimeStamper;
use crate::LogLine;
use std::path::PathBuf;
use crate::indexer::IndexedLog;

use crate::files::{LogBase, LogSource, new_text_file};

/**
 * Log is an adapter interface used to instantiate a SaneIndexer from different kinds of LogSources.
 */
pub struct Log {
    pub(crate) file: SaneIndexer<LogSource>,
    pub(crate) format: TimeStamper,
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
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }
}

// Constructors
impl Log {
    pub fn new(src: SaneIndexer<LogSource>) -> Self {
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    // unused?
    pub fn from_source(file: LogSource) -> Self {
        log::trace!("Instantiate log from LogSource");
        let src = SaneIndexer::new(file);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    pub fn open(file: Option<PathBuf>) -> std::io::Result<Self> {
        log::trace!("Instantiate log from file {:?}", file);
        let src = new_text_file(file)?;
        let log = Log {
            file: SaneIndexer::new(src),
            format: TimeStamper::default(),
        };
        Ok(log)
    }
}

// Navigation
impl IndexedLog for Log {

    /// Position log to read from given offset
    fn seek(&mut self, pos: usize) -> usize {
        self.file.seek(pos)
    }

    #[inline]
    fn next(&mut self) -> Option<LogLine> {
        self.file.next()
    }

    #[inline]
    fn next_back(&mut self) -> Option<LogLine> {
        self.file.next_back()
    }

    #[inline]
    fn len(&self) -> usize {
        self.file.len()
    }

    fn count_lines(&self) -> usize {
        self.file.count_lines()
    }

    fn indexed_bytes(&self) -> usize {
        self.file.indexed_bytes()
    }

    #[inline]
    fn read_line(&mut self, offset: usize) -> (usize, Option<LogLine>) {
        self.file.read_line(offset)
    }
}

// Miscellaneous
impl Log {
    #[inline]
    pub fn wait_for_end(&mut self) {
        log::trace!("Wait for end of file");
        self.file.wait_for_end()
    }
}