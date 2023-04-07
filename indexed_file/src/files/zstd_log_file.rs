// Reader of compressed zstd files

use std::{path::PathBuf, io::Read};

use std::io::{Seek, SeekFrom};
use crate::files::CompressedFile;
use std::fs::File;

use crate::files::LogFileUtil;
use crate::files::LogFileTrait;

use super::text_log::TextLog;

pub struct ZstdLogFile {
    file: TextLog<CompressedFile<File>>,
}

impl ZstdLogFile {
    pub fn new(filename: &PathBuf) -> std::io::Result<ZstdLogFile> {
        let file = File::open(filename)?;
        if !CompressedFile::is_recognized(&file) {
            Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Unrecognized file type")))
        } else {
            let zf = CompressedFile::new(file)?;
            Ok(ZstdLogFile {
                // file_path: input_file.unwrap(),
                file: TextLog::new(zf),
            })
        }
    }
}

impl LogFileTrait for ZstdLogFile {}

impl LogFileUtil for ZstdLogFile {
    fn len(&self) -> usize {
        self.file.len()
    }

    fn quench(&mut self) {
        self.file.quench();
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        self.file.into_inner().get_chunk(target)
    }
}

impl Read for ZstdLogFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.into_inner_mut().read(buf)
    }
}

impl  Seek for ZstdLogFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.into_inner_mut().seek(pos)
    }
}