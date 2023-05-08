use crate::{time_stamper::TimeStamper, LineIndexerIterator, LineIndexerDataIterator, LogLine};
use std::path::PathBuf;
use crate::indexer::line_indexer::LineIndexer;

use crate::files::{LogBase, LogSource, new_text_file};

pub struct Log {
    pub(crate) file: LineIndexer<LogSource>,
    pub(crate) format: TimeStamper,
}

impl<LOG: LogBase + 'static> From<LOG> for Log {
    fn from(file: LOG) -> Self {
        let src = LogSource::from(file);
        Self::from(src)
    }
}

impl From<LogSource> for Log {
    fn from(src: LogSource) -> Self {
        let src = LineIndexer::new(src);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }
}

impl Log {
    pub fn new(src: LineIndexer<LogSource>) -> Self {
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    pub fn from_source(file: LogSource) -> Self {
        let src = LineIndexer::new(file);
        Self {
            file: src,
            format: TimeStamper::default(),
        }
    }

    pub fn open(file: Option<PathBuf>) -> std::io::Result<Self> {
        let src = new_text_file(file)?;
        let log = Log {
            file: LineIndexer::new(src),
            format: TimeStamper::default(),
        };
        Ok(log)
    }

    pub fn count_lines(&self) -> usize {
        self.file.count_lines()
    }

    fn iter(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        LineIndexerIterator::new(self)
    }

    pub fn iter_offsets(&mut self) -> impl DoubleEndedIterator<Item = usize> + '_ {
        self.iter()
    }

    pub fn iter_lines<'a>(&'a mut self) -> impl DoubleEndedIterator<Item = LogLine> + 'a {
        LineIndexerDataIterator::new(LineIndexerIterator::new(self))
    }

    pub fn iter_lines_from(&mut self, offset: usize) -> impl DoubleEndedIterator<Item = LogLine> + '_ {
        LineIndexerDataIterator::new(LineIndexerIterator::new_from(self, offset))
    }
}
