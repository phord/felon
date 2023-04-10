// Reader of compressed zstd files

use std::{path::PathBuf, io::Read};

use std::io::{Seek, SeekFrom, BufReader, BufRead};
use crate::files::CompressedFile;
use std::fs::File;

use crate::files::LogFileUtil;
use crate::files::LogFileTrait;

use super::Stream;

pub struct ZstdLogFile {
    file: CompressedFile<BufReader<File>>,
}

impl ZstdLogFile {
    pub fn new(filename: &PathBuf) -> std::io::Result<ZstdLogFile> {
        let file = File::open(filename)?;
        if !CompressedFile::is_recognized(&file) {
            Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Unrecognized file type")))
        } else {
            let file = BufReader::new(file);
            let zf = CompressedFile::new(file)?;
            Ok(ZstdLogFile {
                // file_path: input_file.unwrap(),
                file: zf,
            })
        }
    }
}

impl LogFileTrait for ZstdLogFile {}

impl LogFileUtil for ZstdLogFile {
    fn len(&self) -> usize {
        self.file.get_length()
    }

    fn quench(&mut self) {
        self.file.wait();
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        self.file.get_chunk(target)
    }
}

impl Read for ZstdLogFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl BufRead for ZstdLogFile {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.file.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.file.consume(amt)
    }
}

impl  Seek for ZstdLogFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}