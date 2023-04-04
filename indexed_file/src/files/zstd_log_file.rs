// Reader of compressed zstd files

use std::path::PathBuf;

use crate::files::CompressedFile;
use std::fs::File;

use crate::files::LogFileTrait;
use crate::files::Stream;

use super::text_log_file::TextLog;

impl<R> Stream for CompressedFile<R> {
    fn len(&self) -> usize {
        self.len() as usize
    }
    // Wait on any data at all; Returns true if file is still open
    fn wait(&mut self) -> bool {
        true
    }
}

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

impl LogFileTrait for ZstdLogFile {
    fn len(&self) -> usize {
        self.file.len()
    }

    fn quench(&mut self) {
        self.file.quench();
    }

    fn read(&mut self, offset: usize, len: usize) -> Option<Vec<u8>> {
        self.file.read(offset, len)
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        self.file.chunk(target)
    }
}
