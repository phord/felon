// Reader of unseekable text streams
// For a stream we have to store old lines in RAM to be able to seek around.

use std::path::PathBuf;

use crate::files::LogFileTrait;
use crate::files::CachedStreamReader;
use crate::files::text_log_file::TextLog;

pub struct TextLogStream {
    stream: TextLog<CachedStreamReader>,
}

impl TextLogStream {
    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<TextLogStream> {
        let bfr = CachedStreamReader::new(input_file);
        Ok(TextLogStream {
            stream: TextLog::new(bfr),
        })
    }

}

impl LogFileTrait for TextLogStream {
    fn len(&self) -> usize {
        self.stream.len()
    }

    fn quench(&mut self) {
        self.stream.quench();
    }

    fn read(&mut self, offset: usize, len: usize) -> Option<Vec<u8>> {
        self.stream.read(offset, len)
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        self.stream.chunk(target)
    }
}
