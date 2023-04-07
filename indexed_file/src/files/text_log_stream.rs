// Reader of unseekable text streams
// For a stream we have to store old lines in RAM to be able to seek around.

use std::io::Read;
use std::io::Seek;
use std::path::PathBuf;

use crate::files::LogFileUtil;
use crate::files::CachedStreamReader;
use crate::files::text_log_file::TextLog;

use super::LogFileTrait;

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

impl LogFileTrait for TextLogStream {}

impl LogFileUtil for TextLogStream {
    fn len(&self) -> usize {
        self.stream.len()
    }

    fn quench(&mut self) {
        self.stream.quench();
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        self.stream.chunk(target)
    }
}

impl Read for TextLogStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.into_inner_mut().read(buf)
    }
}

impl  Seek for TextLogStream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.stream.into_inner_mut().seek(pos)
    }
}