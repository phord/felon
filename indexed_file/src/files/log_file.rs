// Generic wrapper of different readable file types

use std::fs::File;
use std::io::BufRead;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::PathBuf;

use crate::files::CursorLogFile;
use crate::files::MockLogFile;
use crate::files::TextLogFile;
use crate::files::TextLogStream;
use crate::files::ZstdLogFile;


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
impl LogBase for CursorLogFile {}
impl LogBase for MockLogFile {}
impl LogBase for TextLogFile {}
impl LogBase for TextLogStream {}
impl LogBase for ZstdLogFile {}

pub trait LogFile: LogFileUtil + BufRead + Seek {

    // Read a line from a given offset
    fn read_line_at(&mut self, start: usize) -> std::io::Result<String> {
        self.seek(SeekFrom::Start(start as u64))?;

        // FIXME: We strip invalid utf-8 data from the file here. But we should probably do this higher up the chain.
        // Not this from_utf8_lossy means we can't pass binary files through our toy cat tool. Not a goal, but worth knowing.

        let mut buf = vec![];
        match self.read_until(b'\n', &mut buf) {
            Ok(_) => Ok(String::from_utf8_lossy(&buf).into_owned()),
            Err(e) => Err(e),
        }
    }
}

impl LogFile for LogSource {}

pub fn new_text_file(input_file: Option<PathBuf>) -> std::io::Result<LogSource> {
    if let Some(input_file) = input_file {
            // Is it a file?
        let metadata = input_file.metadata()?;
        if metadata.is_file() {
            if let Ok(file) = ZstdLogFile::from_path(&input_file) {
                // FIXME: If the first magic number succeeded but some later error occurred during scan, treat the
                //        file as a compressed file anyway.
                Ok(file.to_src())
            } else {
                let file = File::open(&input_file).unwrap();
                let file = TextLogFile::new(file);
                Ok(file.to_src())
            }
        } else {
            // Must be a stream.  We can't seek in streams.
            let mut file = File::open(&input_file)?;
            assert!(file.seek(SeekFrom::Start(0)).is_err());
            let file = TextLogStream::new(Some(input_file))?;
            Ok(file.to_src())
        }
    } else {
        let file = TextLogStream::new(None)?;
        Ok(file.to_src())
    }
}

pub fn new_mock_file(fill: &str, size: usize, chunk_size: usize) -> LogSource {
    let file = MockLogFile::new(fill.to_string(), size, chunk_size);
    Box::new(file)
}

impl LogFileUtil for LogSource {
    #[inline(always)] fn len(&self) -> usize { self.as_ref().len() }
    #[inline(always)] fn chunk(&self, target: usize) -> (usize, usize) { self.as_ref().chunk(target) }
    #[inline(always)] fn quench(&mut self) { self.as_mut().quench() }
    #[inline(always)] fn wait_for_end(&mut self) { self.as_mut().wait_for_end() }
}

pub trait LogFileUtil {
    fn len(&self) -> usize;
    // Determine the preferred chunk to read to include the target offset
    fn chunk(&self, target: usize) -> (usize, usize) {
        let chunk_size = 1024 * 1024;
        let start = target.saturating_sub(chunk_size / 2);
        let end = (start + chunk_size).min(self.len());
        let start = end.saturating_sub(chunk_size);
        (start, end)
    }

    // Check for more data in file and update state
    fn quench(&mut self) -> ();
    fn wait_for_end(&mut self) {}
}
