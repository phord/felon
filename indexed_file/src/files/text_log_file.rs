// Reader of text files

use std::path::PathBuf;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::files::LogFileUtil;
use crate::files::Stream;

use super::LogFileTrait;

impl Stream for File {
    fn len(&self) -> usize {
        self.metadata().unwrap().len() as usize
    }
    // Wait on any data at all; Returns true if file is still open
    fn wait(&mut self) -> bool {
        true
    }
}

pub struct TextLog<T> {
    file: T,
}

impl<T: Read + Seek + Stream> LogFileTrait for TextLog<T> {}

impl<T: Read + Stream + Seek> LogFileUtil for TextLog<T> {
    fn len(&self) -> usize {
        self.file.len()
    }

    fn quench(&mut self) {
        self.file.wait();
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        let chunk_size = 1024 * 1024;
        let start = target.saturating_sub(chunk_size / 2);
        let end = (start + chunk_size).min(self.len());
        let start = end.saturating_sub(chunk_size);
        (start, end)
    }
}

impl<T: Read> Read for TextLog<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl<T: Seek> Seek for TextLog<T> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl<T> TextLog<T> {
    pub fn new(file: T) -> Self {
        Self {
            file
        }
    }

    pub fn into_inner(&self) -> &T {
        &self.file
    }

    pub fn into_inner_mut(&mut self) -> &mut T {
        &mut self.file
    }
}


pub struct TextLogFile {
    file: TextLog<File>,
}

impl TextLogFile {
    pub fn new(filename: &PathBuf) -> std::io::Result<TextLogFile> {
        Ok(TextLogFile {
            // file_path: input_file.unwrap(),
            file: TextLog::new(File::open(filename)?),
        })
    }
}

impl LogFileTrait for TextLogFile {}

impl LogFileUtil for TextLogFile {
    fn len(&self) -> usize {
        self.file.len()
    }

    fn quench(&mut self) {
        self.file.quench();
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        self.file.chunk(target)
    }
}


impl Read for TextLogFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.into_inner().read(buf)
    }
}

impl Seek for TextLogFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.into_inner().seek(pos)
    }
}