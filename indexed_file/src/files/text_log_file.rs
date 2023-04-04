// Reader of text files

use std::path::PathBuf;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::files::LogFileTrait;
use crate::files::Stream;

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

impl<T: Read + Stream + Seek> LogFileTrait for TextLog<T> {
    fn len(&self) -> usize {
        self.file.len()
    }

    fn quench(&mut self) {
        self.file.wait();
    }

    fn read(&mut self, offset: usize, len: usize) -> Option<Vec<u8>> {
        if offset > self.len() {
            None
        } else {
            let end = (offset + len).min(self.len());
            let mut buf = vec![0u8; end-offset];
            match self.file.seek(SeekFrom::Start(offset as u64)) {
                Err(_) => None,
                Ok(_pos) => {
                    match self.file.read(&mut buf) {
                        Err(_) => None,  // TODO: Log an error somewhere?
                        Ok(actual) => {
                            assert!(actual <= len);
                            buf.truncate(actual);
                            Some(buf)
                        },
                    }
                }
            }
        }
    }

    fn chunk(&self, target: usize) -> (usize, usize) {
        let chunk_size = 1024 * 1024;
        let start = target.saturating_sub(chunk_size / 2);
        let end = (start + chunk_size).min(self.len());
        let start = end.saturating_sub(chunk_size);
        (start, end)
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

impl LogFileTrait for TextLogFile {
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
