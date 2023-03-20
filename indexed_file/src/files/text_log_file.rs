// Reader of text files

use std::path::PathBuf;

use std::fs::File;
use std::fmt;
use mapr::{MmapOptions, Mmap};

use crate::files::LogFileTrait;

pub struct TextLogFile {
    // pub file_path: PathBuf,
    mmap: Mmap,
}

impl fmt::Debug for TextLogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextLogFile")
         .field("bytes", &self.len())
         .finish()
    }
}

impl LogFileTrait for TextLogFile {
    fn len(&self) -> usize {
        self.mmap.len()
    }

    fn read(&self, offset: usize, len: usize) -> Option<&[u8]> {
        if offset > self.len() {
            None
        } else {
            let end = (offset + len).min(self.len());
            Some(&self.mmap[offset..end])
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

use std::io::{Error, ErrorKind};
impl TextLogFile {

    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<TextLogFile> {
        let file = if let Some(file_path) = input_file {
            // Must have a filename as input.
            let file = File::open(file_path)?;
            Some(file)
        } else {
            // Print error.
            eprintln!("Expected '<input>' or input over stdin.");
            return Err(Error::new(ErrorKind::Other, "Expected a filename"));
        };

        let mmap = unsafe { MmapOptions::new().map(&file.unwrap()) };
        let mmap = mmap.expect("Could not mmap file.");

        let file = TextLogFile {
            // file_path: input_file.unwrap(),
            mmap,
        };

        Ok(file)
    }
}
