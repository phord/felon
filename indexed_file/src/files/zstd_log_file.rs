// Reader of compressed zstd files

use std::path::PathBuf;
use std::io::BufReader;
use crate::files::CompressedFile;
use std::fs::File;
use crate::files::LogFile;


pub type ZstdLogFile = CompressedFile<BufReader<File>>;

impl ZstdLogFile {
    pub fn from_path(filename: &PathBuf) -> std::io::Result<ZstdLogFile> {
        let file = File::open(filename)?;
        if !CompressedFile::is_recognized(&file) {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Unrecognized file type".to_string()))
        } else {
            let file = BufReader::new(file);
            let zf = CompressedFile::new(file)?;
            Ok(zf)
        }
    }
}

impl LogFile for ZstdLogFile {}
