// Mock log file helper

use std::fmt;
use crate::log_file::LogFileTrait;

pub struct MockLogFile {
    filler: String,
    size: usize,
    buffer: String,
}

impl fmt::Debug for MockLogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockLogFile")
         .field("filler", &self.filler)
         .field("bytes", &self.len())
         .finish()
    }
}

impl LogFileTrait for MockLogFile {
    fn len(&self) -> usize {
        self.size
    }

    fn read(&self, offset: usize, len: usize) -> Option<&[u8]> {
        if offset > self.len() {
            None
        } else {
            let offset = offset % self.filler.len();
            let end = (offset + len).min(self.len());
            assert!(end < self.buffer.len());
            Some(self.buffer[offset..end].as_bytes())
        }
    }
}

impl MockLogFile {

    pub fn new(fill: String, size: usize) -> MockLogFile {
        assert!(fill.len() > 0);
        let copies = 1024 * 1024 * 16 / fill.len() + 1;
        let buffer = (0..copies)
            .map(|_| fill.as_str())
            .collect::<Vec<&str>>()
            .join("");

        MockLogFile {
            filler: fill,
            size,
            buffer,
        }
    }
}




// Tests for MockLogFile
#[cfg(test)]
mod tests {
    use crate::log_file::LogFile;
    use crate::log_file::LogFileTrait;

    #[test]
    fn test_mock_log_file_basic() {
        let size = 16 * 1024;
        let file = LogFile::new_mock_file("fill", size);
        assert_eq!(file.len(), size);
    }

    #[test]
    fn test_mock_log_file_read_basic() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let file = LogFile::new_mock_file(fill, size);
        assert_eq!(file.read(0, 10), Some(fill[..10].as_bytes()));
    }

    #[test]
    fn test_mock_log_file_read_offset() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let file = LogFile::new_mock_file(fill, size);
        let offset = fill.len() * 10;
        assert_eq!(file.read(offset, 10), Some(fill[..10].as_bytes()));
    }

    #[test]
    fn test_mock_log_file_read_multiline() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let file = LogFile::new_mock_file(fill, size);
        let mut ret = fill.to_string();
        ret.push_str(&fill[..]);
        let offset = fill.len() * 10;
        let len = ret.len();

        assert_eq!(file.read(offset, len), Some(ret.as_bytes()));
    }

    #[test]
    fn test_mock_log_file_read_multiline_middle() {
        let size = 16 * 1024;
        let fill = "this is a test\n";
        let file = LogFile::new_mock_file(fill, size);

        let ofs = fill.len()/2;
        let end = fill.len() - 1;
        let mut ret = fill[ofs..].to_string();
        ret.push_str(&fill[..end - ofs]);
        let offset = fill.len() * 10 + ofs;
        let len = ret.len();

        assert_eq!(file.read(offset, len), Some(ret.as_bytes()));
    }

}
