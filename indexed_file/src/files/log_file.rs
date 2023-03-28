// Generic wrapper of different readable file types

use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;

use crate::files::MockLogFile;
use crate::files::TextLogFile;
use crate::files::TextLogStream;


pub struct LogFile {
    file: Box<dyn LogFileTrait>,
}

impl LogFile {

    pub fn new_text_file(input_file: Option<PathBuf>) -> std::io::Result<LogFile> {
        if let Some(input_file) = input_file {
                // Is it a file?
            let metadata = input_file.metadata()?;
            println!("{:?}", &metadata);
            if metadata.is_file() {
                let file = TextLogFile::new(Some(input_file))?;
                Ok(LogFile {
                    file: Box::new(file),
                })
            } else {
                // Must be a stream.  We can't seek in streams.
                let mut file = File::open(&input_file)?;
                assert!(file.seek(SeekFrom::Start(0)).is_err());
                let file = TextLogStream::new(Some(input_file))?;
                Ok(LogFile {
                    file: Box::new(file),
                })
            }
        } else {
            todo!("Implement read from stdin");
        }
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
    fn read(&mut self, offset: usize, len: usize) -> Option<Vec<u8>> { self.file.read(offset, len) }
    fn chunk(&self, target: usize) -> (usize, usize) { self.file.chunk(target) }
}

// generic representation of text we can show in our pager
pub trait LogFileTrait {
    fn len(&self) -> usize;
    // TODO: return a String from everywhere, and require that strings are valid utf8
    fn read(&mut self, offset: usize, len: usize) -> Option<Vec<u8>>;
    // Determine the preferred chunk to read to include the target offset
    fn chunk(&self, target: usize) -> (usize, usize);
}
