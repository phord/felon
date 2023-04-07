// Wrapper for text things

use std::io::{Read, Seek, SeekFrom};

use crate::files::LogFileUtil;
use crate::files::Stream;

use super::LogFileTrait;

pub struct TextLog<T> {
    file: T,
}

impl<T: Read + Seek + Stream> LogFileTrait for TextLog<T> {}

impl<T: Read + Stream + Seek> LogFileUtil for TextLog<T> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.file.len()
    }

    #[inline(always)]
    fn quench(&mut self) {
        self.file.wait();
    }
}

impl<T: Read> Read for TextLog<T> {
    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl<T: Seek> Seek for TextLog<T> {
    #[inline(always)]
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

    #[inline(always)]
    pub fn into_inner(&self) -> &T {
        &self.file
    }

    #[inline(always)]
    pub fn into_inner_mut(&mut self) -> &mut T {
        &mut self.file
    }
}
