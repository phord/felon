// Reader of unseekable text streams
// For a stream we have to store old lines in RAM to be able to seek around.

use std::path::PathBuf;
use std::fmt;
use std::io::{Read, Seek, SeekFrom};

use crate::files::LogFileTrait;
use crate::files::AsyncStdin;

pub struct TextLogStream {
    // pub file_path: PathBuf,
    stream: AsyncStdin,
}

impl fmt::Debug for TextLogStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextLogStream")
         .field("bytes", &self.len())
         .finish()
    }
}

impl LogFileTrait for TextLogStream {
    fn len(&self) -> usize {
        self.stream.len()
    }

    fn quench(&mut self) {
        println!("quench");
        self.stream.wait();
    }

    fn read(&mut self, offset: usize, len: usize) -> Option<Vec<u8>> {
        if offset > self.len() {
            None
        } else {
            let end = (offset + len).min(self.len());
            let mut buf = vec![0u8; end-offset];
            let _actual = self.stream.read(&mut buf).unwrap();
            match self.stream.seek(SeekFrom::Start(offset as u64)) {
                Err(_) => None,
                Ok(_pos) => {
                    match self.stream.read(&mut buf) {
                        Err(_) => None,  // TODO: Log an error somewhere?
                        Ok(actual) => {
                            assert!(actual <= len);
                            buf.truncate(actual);
                            Some(buf)
                        },
                    }
                }
            }
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

impl TextLogStream {
    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<TextLogStream> {
        let bfr = AsyncStdin::new(input_file);
        Ok(TextLogStream {
            stream: bfr,
        })
    }

}
