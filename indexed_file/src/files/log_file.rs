// Generic wrapper of different readable file types

use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::BufRead;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;

use crate::files::MockLogFile;
use crate::files::TextLogFile;
use crate::files::TextLogStream;
use crate::files::ZstdLogFile;


pub struct LogFile {
    file: Box<dyn LogFileTrait>,
}

pub trait LogFileTrait: LogFileUtil + BufRead + Seek {}

impl LogFileTrait for LogFile {}

impl LogFile {

    pub fn new_text_file(input_file: Option<PathBuf>) -> std::io::Result<LogFile> {
        if let Some(input_file) = input_file {
                // Is it a file?
            let metadata = input_file.metadata()?;
            if metadata.is_file() {
                if let Ok(file) = ZstdLogFile::new(&input_file) {
                    // FIXME: If the first magic number succeeded but some later error occurred during scan, treat the
                    //        file as a compressed file anyway.
                    Ok(LogFile {
                        file: Box::new(file),
                    })
                } else {
                    let file = File::open(&input_file).unwrap();
                    let file = BufReader::new(file);
                    let file = TextLogFile::new(file)?;
                    Ok(LogFile {
                        file: Box::new(file),
                    })
                }
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
            let file = TextLogStream::new(None)?;
            Ok(LogFile {
                file: Box::new(file),
            })
        }
    }

    pub fn new_mock_file(fill: &str, size: usize, chunk_size: usize) -> LogFile {
        let file = MockLogFile::new(fill.to_string(), size, chunk_size);
        LogFile {
            file: Box::new(file),
        }
    }
}

// TODO: Make LogFileTrait wrappers implement ReadBuf instead of Read
impl LogFileUtil for LogFile {
    #[inline(always)] fn len(&self) -> usize { self.file.len() }
    #[inline(always)] fn chunk(&self, target: usize) -> (usize, usize) { self.file.chunk(target) }
    #[inline(always)] fn quench(&mut self) { self.file.quench() }
}

// generic representation of text we can show in our pager
pub trait LogFileUtil {
    fn len(&self) -> usize;
    // Determine the preferred chunk to read to include the target offset
    fn chunk(&self, target: usize) -> (usize, usize) {
        let chunk_size = 1024 * 1024;
        let start = target.saturating_sub(chunk_size / 2);
        let end = (start + chunk_size).min(self.len());
        let start = end.saturating_sub(chunk_size);
        (start, end)
    }

    // Check for more data in file and update state
    fn quench(&mut self) -> ();
}

impl Read for LogFile {
    #[inline(always)] fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for LogFile {
    #[inline(always)] fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl BufRead for LogFile {
    #[inline(always)]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.file.fill_buf()
    }

    #[inline(always)]
    fn consume(&mut self, amt: usize) {
        self.file.consume(amt)
    }
}
