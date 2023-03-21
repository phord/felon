// Generic wrapper of different readable file types

use std::path::PathBuf;

use crate::files::MockLogFile;
use crate::files::TextLogFile;


pub struct LogFile {
    file: Box<dyn LogFileTrait>,
}

impl LogFile {
    pub fn new_text_file(input_file: Option<PathBuf>) -> std::io::Result<LogFile> {
        let file = TextLogFile::new(input_file)?;
        Ok(LogFile {
            file: Box::new(file),
        })
    }

    pub fn new_mock_file(fill: &str, size: usize, chunk_size: usize) -> LogFile {
        let file = MockLogFile::new(fill.to_string(), size, chunk_size);
        LogFile {
            file: Box::new(file),
        }
    }
}

impl LogFileTrait for LogFile {
    fn len(&self) -> usize { self.file.len() }
    fn read(&self, offset: usize, len: usize) -> Option<&[u8]> { self.file.read(offset, len) }
    fn chunk(&self, target: usize) -> (usize, usize) { self.file.chunk(target) }
}

// generic representation of text we can show in our pager
pub trait LogFileTrait {
    fn len(&self) -> usize;
    fn read(&self, offset: usize, len: usize) -> Option<&[u8]>;
    // Determine the preferred chunk to read to include the target offset
    fn chunk(&self, target: usize) -> (usize, usize);
}
