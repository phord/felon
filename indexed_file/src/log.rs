use crate::indexer::eventual_index::Location;
use crate::time_stamper::TimeStamper;
use crate::LogLine;
use std::path::PathBuf;
use crate::indexer::line_indexer::{IndexedLog, LineIndexer, LogLocation};

use crate::files::{LogBase, LogSource, new_text_file};

/**
 * Log is an adapter interface used to instantiate a LineIndexer from different kinds of LogSources.
 */
pub struct Log {
    pub(crate) file: LineIndexer<LogSource>,
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
        let src = LineIndexer::new(src);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }
}

// Constructors
impl Log {
    pub fn new(src: LineIndexer<LogSource>) -> Self {
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    // unused?
    pub fn from_source(file: LogSource) -> Self {
        log::trace!("Instantiate log from LogSource");
        let src = LineIndexer::new(file);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    pub fn open(file: Option<PathBuf>) -> std::io::Result<Self> {
        log::trace!("Instantiate log from file {:?}", file);
        let src = new_text_file(file)?;
        let log = Log {
            file: LineIndexer::new(src),
            format: TimeStamper::default(),
        };
        Ok(log)
    }
}

// Navigation
impl IndexedLog for Log {

    #[inline]
    fn next(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        self.file.next(pos)
    }

    #[inline]
    fn next_back(&mut self, pos: &mut LogLocation) -> Option<LogLine> {
        self.file.next_back(pos)
    }

    #[inline]
    fn len(&self) -> usize {
        self.file.len()
    }

    fn count_lines(&self) -> usize {
        self.file.count_lines()
    }

    #[inline]
    fn read_line(&mut self, pos: &mut LogLocation, next_pos: Location) -> Option<LogLine> {
        self.file.read_line(pos, next_pos)
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