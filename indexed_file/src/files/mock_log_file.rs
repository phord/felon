// Mock log file helper

use std::io::BufRead;
use std::{fmt, io::Read};
use crate::files::LogFile;

pub struct MockLogFile {
    filler: String,
    size: usize,
    pos: u64,
    buffer: String,
    pub chunk_size: usize,
}

impl fmt::Debug for MockLogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockLogFile")
         .field("filler", &self.filler)
         .field("bytes", &self.len())
         .finish()
    }
}

impl Stream for MockLogFile {
    fn len(&self) -> usize {
        self.size
    }

    fn poll(&mut self, _timeout: Option<std::time::Instant>) -> usize { self.len() }
    fn is_open(&self) -> bool { false }
}

impl LogFile for MockLogFile {
    fn chunk(&self, target: usize) -> (usize, usize) {
        let start = target.saturating_sub(self.chunk_size / 2);
        let end = (start + self.chunk_size).min(self.len());
        let start = end.saturating_sub(self.chunk_size);
        (start, end)
    }
}

impl Read for MockLogFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len();
        if self.pos as usize > self.len() {
            Ok(0)
        } else {
            let offset = self.pos as usize % self.filler.len();
            let end = (offset + len).min(self.len());
            assert!(end < self.buffer.len());
            buf.copy_from_slice(self.buffer[offset..end].as_bytes());
            Ok(end-offset)
        }
    }
}

impl BufRead for MockLogFile {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.pos as usize > self.len() {
            Ok(&self.filler.as_bytes()[..0])
        } else {
            let offset = self.pos as usize % self.filler.len();
            let len = (self.filler.len() - offset).min(self.len() - self.pos as usize);
            Ok(&self.filler.as_bytes()[offset..offset+len])
        }
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt as u64
    }
}

use std::io::{Seek, SeekFrom};

use super::Stream;
impl Seek for MockLogFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let (start, offset) = match pos {
            SeekFrom::Start(n) => (0_i64, n as i64),
            SeekFrom::Current(n) => (self.pos as i64, n),
            SeekFrom::End(n) => (self.len() as i64, n),
        };
        self.pos = ((start.saturating_add(offset)) as u64).min(self.len() as u64);
        Ok(self.pos)
    }
}
impl MockLogFile {

    pub fn new(fill: String, size: usize, chunk_size: usize) -> MockLogFile {
        assert!(!fill.is_empty());
        let copies = 1024 * 1024 * 16 / fill.len() + 1;
        let buffer = (0..copies)
            .map(|_| fill.as_str())
            .collect::<Vec<&str>>()
            .join("");

        MockLogFile {
            filler: fill,
            size,
            pos: 0,
            chunk_size,
            buffer,
        }
    }
}




// Tests for MockLogFile
#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::io::{Seek, SeekFrom};

    use crate::files::LogSource;
    use crate::files::new_mock_file;

    fn old_read(file: &mut LogSource, offset: usize, len: usize ) -> Option<Vec<u8>> {
        file.seek(SeekFrom::Start(offset as u64)).expect("Seek never fails");
        let mut buf = vec![0u8; len];
        match file.read(&mut buf) {
            Ok(_bytes) => Some(buf),
            _ => None,
        }
    }

    #[test]
    fn test_mock_log_file_basic() {
        let size = 16 * 1024;
        let file = new_mock_file("fill", size, 100);
        assert_eq!(file.len(), size);
    }

    #[test]
    fn test_mock_log_file_read_basic() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let mut file = new_mock_file(fill, size, 100);
        assert_eq!(old_read(&mut file, 0, 10), Some(fill[..10].as_bytes().to_vec()));
    }

    #[test]
    fn test_mock_log_file_read_offset() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let mut file = new_mock_file(fill, size, 100);
        let offset = fill.len() * 10;
        assert_eq!(old_read(&mut file, offset, 10), Some(fill[..10].as_bytes().to_vec()));
    }

    #[test]
    fn test_mock_log_file_read_multiline() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let mut file = new_mock_file(fill, size, 100);
        let mut ret = fill.to_string();
        ret.push_str(fill);
        let offset = fill.len() * 10;
        let len = ret.len();

        assert_eq!(old_read(&mut file, offset, len), Some(ret.as_bytes().to_vec()));
    }

    #[test]
    fn test_mock_log_file_read_multiline_middle() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let mut file = new_mock_file(fill, size, 100);

        let ofs = fill.len()/2;
        let end = fill.len() - 1;
        let mut ret = fill[ofs..].to_string();
        ret.push_str(&fill[..end - ofs]);
        let offset = fill.len() * 10 + ofs;
        let len = ret.len();

        assert_eq!(old_read(&mut file, offset, len), Some(ret.as_bytes().to_vec()));
    }

    #[test]
    fn test_mock_log_file_chunk_sizes() {
        let size = 3 * 1024;
        let fill = "this is a test\n";
        let file = new_mock_file(fill, size, 100);

        for i in 0..file.len() {
            let (start, end) = file.chunk(i);
            println!("{}: {} - {}", i, start, end);
            assert!(start <= i);
            assert!(i <= end);
            assert_eq!(end - start, 100);
        }
    }


}
