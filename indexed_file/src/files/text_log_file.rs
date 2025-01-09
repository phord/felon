// Reader of regular text files

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek};

use crate::files::Stream;

pub struct TextLogFile {
    file: BufReader<File>,
    len: usize,
    pos: u64,
}

impl TextLogFile {
    pub fn new(file: BufReader<File>) -> std::io::Result<TextLogFile> {
        let len = file.get_ref().metadata()?.len() as usize;
        Ok(TextLogFile { file, len, pos: 0})
    }
}

impl Stream for TextLogFile {
    #[inline(always)]
    fn get_length(&self) -> usize {
        self.len
    }
    // Wait on any data at all; Returns true if file is still open
    #[inline(always)]
    fn wait(&mut self) -> bool {
        true
    }
}

impl BufRead for TextLogFile {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.file.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt as u64;
        self.file.consume(amt);
    }
}

impl Read for TextLogFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = self.file.read(buf)?;
        // self.pos += bytes as u64;
        Ok(bytes)
    }
}

impl Seek for TextLogFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        if let std::io::SeekFrom::Start(n) = pos {
            if self.pos == n {
                return Ok(n)
            }
            self.pos = n;
        }
        self.file.seek(pos)
    }
}