// Structs to index lines in a text file
// TODO: Cleanup - This is a clone of indexer (LogFileTrait) that doesn't parse out words and numbers.  It only parses lines.
//       Needs to be allowed to run in the background better, in a way that Rust can accept.

use std::path::PathBuf;

use std::fs::File;
use std::fmt;
use mapr::{MmapOptions, Mmap};
use crate::mock_log_file::MockLogFile;

enum DataSource {
    NullFile,
    TextFile(TextLogFile),
    MockFile(MockLogFile),
}

pub struct LogFile {
    file: DataSource,
}

impl LogFile {
    pub fn new_text_file(input_file: Option<PathBuf>) -> std::io::Result<LogFile> {
        let file = TextLogFile::new(input_file)?;
        Ok(LogFile {
            file: DataSource::TextFile(file),
        })
    }

    pub fn new_mock_file(fill: &str, size: usize, chunk_size: usize) -> LogFile {
        let file = MockLogFile::new(fill.to_string(), size, chunk_size);
        LogFile {
            file: DataSource::MockFile(file),
        }
    }
}

impl LogFileTrait for LogFile {
    fn len(&self) -> usize {
        match &self.file {
            DataSource::TextFile(file) => file.len(),
            DataSource::MockFile(file) => file.len(),
            _ => unimplemented!(),
        }
    }

    fn read(&self, offset: usize, len: usize) -> Option<&[u8]> {
        match &self.file {
            DataSource::TextFile(file) => file.read(offset, len),
            DataSource::MockFile(file) => file.read(offset, len),
            _ => unimplemented!(),
        }
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        match &self.file {
            DataSource::TextFile(file) => file.chunk(target),
            DataSource::MockFile(file) => file.chunk(target),
            _ => unimplemented!(),
        }
    }
}

// generic representation of text we can show in our pager
pub trait LogFileTrait {
    fn len(&self) -> usize;
    fn read(&self, offset: usize, len: usize) -> Option<&[u8]>;
    // Determine the preferred chunk to read to include the target offset
    fn chunk(&self, target: usize) -> (usize, usize);
}

pub struct TextLogFile {
    // pub file_path: PathBuf,
    mmap: Mmap,
}

impl fmt::Debug for TextLogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextLogFile")
         .field("bytes", &self.len())
         .finish()
    }
}

impl LogFileTrait for TextLogFile {
    fn len(&self) -> usize {
        self.mmap.len()
    }

    fn read(&self, offset: usize, len: usize) -> Option<&[u8]> {
        if offset > self.len() {
            None
        } else {
            let end = (offset + len).min(self.len());
            Some(&self.mmap[offset..end])
        }
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        let chunk_size = 1024 * 1024;
        let start = target.saturating_sub(chunk_size / 2);
        let end = (start + chunk_size).min(self.len());
        let start = end.saturating_sub(chunk_size);
        (start, end)
    }
}

use std::io::{Error, ErrorKind};
impl TextLogFile {

    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<TextLogFile> {
        let file = if let Some(file_path) = input_file {
            // Must have a filename as input.
            let file = File::open(file_path)?;
            Some(file)
        } else {
            // Print error.
            eprintln!("Expected '<input>' or input over stdin.");
            return Err(Error::new(ErrorKind::Other, "Expected a filename"));
        };

        let mmap = unsafe { MmapOptions::new().map(&file.unwrap()) };
        let mmap = mmap.expect("Could not mmap file.");

        let file = TextLogFile {
            // file_path: input_file.unwrap(),
            mmap,
        };

        Ok(file)
    }
}
