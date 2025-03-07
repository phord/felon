// Generic wrapper of different readable file types

use std::fs::File;
use std::io::BufReader;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;

use bstr::io::BufReadExt;

use crate::files::CursorLogFile;
use crate::files::MockLogFile;
use crate::files::TextLogFile;
use crate::files::TextLogStream;
use crate::files::ZstdLogFile;

use super::CachedStreamReader;
use super::Stream;


pub type LogSource = Box<dyn LogFile>;

impl<LOG: LogBase + 'static> From<LOG> for LogSource {
    fn from(file: LOG) -> Self {
        Box::new(file) as LogSource
    }
}

pub trait LogBase: LogFile {
    fn to_src(self) -> LogSource
    where Self: Sized + 'static {
        LogSource::from(self)
    }
}

// All of these can be promoted to LogSource
impl LogFile for CachedStreamReader {}
impl LogFile for TextLogFile {}
impl LogFile for ZstdLogFile {}
impl LogFile for CursorLogFile {}

impl LogBase for CursorLogFile {}
impl LogBase for MockLogFile {}
impl LogBase for TextLogFile {}
impl LogBase for TextLogStream {}
impl LogBase for ZstdLogFile {}

pub trait LogFile: BufReadExt + Seek + Stream {

    // Read a line from a given offset
    fn read_line_at(&mut self, start: usize) -> std::io::Result<String> {
        self.seek(SeekFrom::Start(start as u64))?;

        // We could return this, except it will not handle invalid utf-8 data (and it strips \n)
        // return self.lines().next().unwrap()

        // FIXME: We strip invalid utf-8 data from the file here. But we should probably do this higher up the chain.
        // Note this from_utf8_lossy means we can't pass binary files through our toy cat tool. Not a goal, but worth knowing.

        let mut buf = vec![];
        // FIXME: Does this end early when some utf-8 code sequence inludes 0x10?
        match self.read_until(b'\n', &mut buf) {
            Ok(_) => Ok(String::from_utf8_lossy(&buf).into_owned()),
            Err(e) => Err(e),
        }
    }

    /// Parse a block of data from the file and return the offsets of the lines (byte after each LF)
    /// This is about 3x as fast as read_line_at(), but it doesn't do Unicode conversion and it doesn't return the found lines.
    fn find_lines(&mut self, range: &std::ops::Range<usize>) -> std::io::Result<Vec<usize>>
    where Self: Sized {
        let len = range.len().min(self.len() - range.start).min(10 * 1024 * 1024);
        self.seek(SeekFrom::Start(range.start as u64))?;
        let estimate_avg_line_length = 50;
        let mut lines = Vec::with_capacity(len / estimate_avg_line_length);
        let mut offset = range.start;
        if offset == 0 {
            // There's always a line beginning at zero
            lines.push(0);
        }
        self.for_byte_line_with_terminator(|line| {
            offset += line.len();
            lines.push(offset);
            if offset >= range.start + len {
                return Ok(false);
            }
            Ok(true)
        })?;
        Ok(lines)
    }

    // Determine the preferred chunk to read to include the target offset
    fn chunk(&self, target: usize) -> (usize, usize) {
        let chunk_size = 1024 * 1024;
        let start = target.saturating_sub(chunk_size / 2);
        let end = (start + chunk_size).min(self.len());
        let start = end.saturating_sub(chunk_size);
        (start, end)
    }
}

impl Stream for LogSource {
    #[inline(always)] fn len(&self) -> usize { self.as_ref().len() }
    #[inline(always)] fn poll(&mut self, timeout: Option<std::time::Instant>) -> usize { self.as_mut().poll(timeout) }
    #[inline(always)] fn is_open(&self) -> bool { self.as_ref().is_open() }
}

impl LogFile for LogSource {
    #[inline(always)] fn chunk(&self, target: usize) -> (usize, usize) { self.as_ref().chunk(target) }
    #[inline(always)] fn read_line_at(&mut self, start: usize) -> std::io::Result<String> { self.as_mut().read_line_at(start) }
}

pub fn new_text_file(input_file: Option<&PathBuf>) -> std::io::Result<LogSource> {
    if let Some(input_file) = input_file {
            // Is it a file?
        let metadata = input_file.metadata()?;
        if metadata.is_file() {
            if let Ok(file) = ZstdLogFile::from_path(input_file) {
                // FIXME: If the first magic number succeeded but some later error occurred during scan, treat the
                //        file as a compressed file anyway.
                Ok(file.to_src())
            } else {
                let file = File::open(input_file).unwrap();
                let file = BufReader::new(file);
                let file = TextLogFile::new(file)?;
                Ok(file.to_src())
            }
        } else {
            // Must be a stream.  We can't seek in streams; assert that seek fails to make sure.
            let mut file = File::open(input_file)?;
            assert!(file.seek(SeekFrom::Start(0)).is_err());
            let file = TextLogStream::new(Some(input_file))?;
            Ok(file.to_src())
        }
    } else {
        // Stream from stdin
        let file = TextLogStream::new(None)?;
        Ok(file.to_src())
    }
}

pub fn new_mock_file(fill: &str, size: usize, chunk_size: usize) -> LogSource {
    let file = MockLogFile::new(fill.to_string(), size, chunk_size);
    Box::new(file)
}
