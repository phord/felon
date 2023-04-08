// Reader of regular text files

use std::fs::File;
use std::io::BufReader;

use crate::files::text_log::TextLog;
use super::LogFileTrait;
use crate::files::Stream;

pub type TextLogFile = TextLog<BufReader<File>>;

impl Stream for TextLogFile {
    #[inline(always)]
    fn get_length(&self) -> usize {
        self.into_inner().get_length()
    }
    // Wait on any data at all; Returns true if file is still open
    #[inline(always)]
    fn wait(&mut self) -> bool {
        true
    }
}

impl Stream for BufReader<File> {
    #[inline(always)]
    fn get_length(&self) -> usize {
        self.get_ref().metadata().unwrap().len() as usize
    }
    // Wait on any data at all; Returns true if file is still open
    #[inline(always)]
    fn wait(&mut self) -> bool {
        true
    }
}

impl LogFileTrait for TextLogFile {}
