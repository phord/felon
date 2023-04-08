// Reader of regular text files

use std::path::PathBuf;

use std::fs::File;
use std::io::{Read, Seek};

use crate::files::LogFileUtil;
use crate::files::text_log::TextLog;
use super::LogFileTrait;
use crate::files::Stream;

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

impl Stream for File {
    #[inline(always)]
    fn get_length(&self) -> usize {
        self.metadata().unwrap().len() as usize
    }
    // Wait on any data at all; Returns true if file is still open
    #[inline(always)]
    fn wait(&mut self) -> bool {
        true
    }
}

impl LogFileTrait for TextLogFile {}

impl LogFileUtil for TextLogFile {
    #[inline(always)]
    fn len(&self) -> usize {
        self.file.len()
    }

    #[inline(always)]
    fn quench(&mut self) {
        self.file.quench();
    }

    #[inline(always)]
    fn chunk(&self, target: usize) -> (usize, usize) {
        self.file.chunk(target)
    }
}


impl Read for TextLogFile {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.into_inner().read(buf)
    }
}

impl Seek for TextLogFile {
    #[inline(always)]
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.into_inner().seek(pos)
    }
}