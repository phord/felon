// Reader of compressed gzip files
use flate2::bufread;
use std::path::PathBuf;
use std::io::{BufReader, Read, Seek};
use std::fs::File;

use super::CachedStreamReader;

// FIXME: rename this to GzipCachedLogFile
pub type GzipLogFile = CachedStreamReader;

impl GzipLogFile {
    fn is_recognized(file: &File) -> bool {
        // Check the magic number
        let mut buf = [0; 2];
        let mut file = file;
        file.read_exact(&mut buf).is_ok() && buf == [0x1f, 0x8b]
    }

    pub fn from_path(filename: &PathBuf) -> std::io::Result<GzipLogFile> {
        let mut file = File::open(filename)?;
        if !Self::is_recognized(&file) {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Unrecognized file type".to_string()))
        } else {
            file.seek(std::io::SeekFrom::Start(0))?;
            let file = BufReader::new(file);
            let gzf = bufread::GzDecoder::new(file);
            let gzf = BufReader::new(gzf);
            let gzf = CachedStreamReader::from_reader(gzf)?;
            Ok(gzf)
        }
    }
}
