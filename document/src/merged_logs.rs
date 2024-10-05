// A collection of log lines from multiple log files, blended together by sorting.

// FIXME: Right now they're sorted by line contents, but they should be sorted by timestamp in the future.

use indexed_file::Log;
use indexed_file::LogLine;

#[cfg(test)]
use indexed_file::files::LogBase;
#[cfg(test)]
use indexed_file::indexer::LineIndexer;

/* Thinking:
    TODO: Need a timestamp for each log line so we can sort by timestamp and jump to time offsets.
    Each line is indexed by (doc-index, offset).

    FIXME: Iterators should produce a LineInfo object that contains all the interesting metadata
           we care about:
           struct LineInfo { line: String, filename: &str, doc_offset: usize, file_offset: usize,
                             timestamp: Time}
   For sorting all files:
   Hold a deconstructed EventualIndex-like thing that has
        a map of doc-offset/timestamp -> (file-index, file-offset) for indexed regions
        a set of per-file checkpoints for gaps, where a checkpoint represents the per-file offset of each file at some edge of the gap
        something to tell us which files definitely do not overlap each other (by tasting the start/end of each file in advance and sorting)

    Need to sort by timestamp or to somehow "heal" sort anomalies because
        - Sometimes timestamped lines are interrupted by one or more multi-line chunks
        - Some files start with a different timestamp format for some reason
        - Many pslog lines are slightly disordered (ms-granularity)

 */

// A long-lived collection of Logs
pub struct MergedLogs {
    files: Vec<Log>
}

type Iter<'a> = Box<dyn DoubleEndedIterator<Item = LogLine> + 'a>;

struct LogIter<'a> {
    next: Option<LogLine>,
    prev: Option<LogLine>,
    iter: Iter<'a>,
}

impl<'a> LogIter<'a> {
    fn new(log: &'a mut Log) -> Self {
        Self {
            iter: Box::new(log.iter_lines()),
            next: None,
            prev: None,
        }
    }

    // Return ref to next string unless EOF, else prev string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn peek_next(&mut self) -> &Option<LogLine> {
        if self.next.is_some() || self.advance() {
            &self.next
        } else {
            &self.prev
        }
    }

    // Return ref to prev string unless EOF, else next string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn peek_prev(&mut self) -> &Option<LogLine> {
        if self.prev.is_some() || self.advance_back() {
            &self.prev
        } else {
            &self.next
        }
    }

    fn advance(&mut self) -> bool {
        // FIXME: Return offset to construct a Cursor with
        // Pre-load the next line for peek
        self.next = self.iter.next();
        self.next.is_some()
    }

    fn advance_back(&mut self) -> bool {
        // FIXME: Return offset to construct a Cursor with
        // Pre-load the prev line for peek
        self.prev = self.iter.next_back();
        self.prev.is_some()
    }

    // Return next string unless EOF, else prev string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn take_next(&mut self) -> Option<LogLine> {
        if self.next.is_none() {
            // No next line to peek.  Maybe we're not initialized.
            self.advance();
        }
        if let Some(next) = self.next.take() {
            self.advance();
            Some(next)
        } else if self.prev.is_some() {
            self.take_prev()
        } else {
            None
        }
    }

    // Return prev string unless EOF, else next string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn take_prev(&mut self) -> Option<LogLine> {
        if self.prev.is_none() {
            // No next line to peek.  Maybe we're not initialized.
            self.advance_back();
        }
        if let Some(prev) = self.prev.take() {
            self.advance_back();
            Some(prev)
        } else if self.next.is_some() {
            self.take_next()
        } else {
            None
        }
    }

}

// A semi-sorted iterator over MergedLogs
pub(crate) struct MergedLogsIterator<'a> {
    // A vector of iterators over lines in multiple files
    iters: Vec<LogIter<'a>>,
}

impl<'a> MergedLogsIterator<'a> {
    pub(crate) fn new(doc: &'a mut MergedLogs) -> Self {
        Self {
            iters: doc.files
                    .iter_mut()
                    .map(LogIter::new)
                    .collect(),
        }
    }
}


impl<'a> Iterator for MergedLogsIterator<'a> {
    type Item = LogLine;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, _line)) = self.iters
            .iter_mut()
            .map(|iter| iter.peek_next())
            .enumerate()
            .filter(|(_, v)| v.is_some())
            .min_by(|(_, line0), (_, line1)| line0.cmp(line1)) {
                // We found a minimum line
                self.iters[i].take_next()
        } else {
            // We ran out of lines
            None
        }
    }
}

impl<'a> DoubleEndedIterator for MergedLogsIterator<'a> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        // Find and return the max current line from all our iterators
        if let Some((i, _line)) = self.iters
            .iter_mut()
            .map(|iter| iter.peek_prev())
            .enumerate()
            .filter(|(_, v)| v.is_some())
            .max_by(|(_, line0), (_, line1)| line0.cmp(line1)) {
                // We found a maximum line
                self.iters[i].take_prev()
        } else {
            // We ran out of lines
            None
        }
    }
}


impl MergedLogs {
    pub fn new() -> Self {
        MergedLogs { files: Vec::default() }
    }

    pub fn push(&mut self, log: Log) {
        self.files.push(log);
    }

    #[cfg(test)]
    pub fn push_logbase<L: LogBase + 'static>(&mut self, log: L) {
        let log = Log::new(LineIndexer::new(log.to_src()));
        self.files.push(log);
    }

    // pub fn new(files: Vec<PathBuf>) -> std::io::Result<Self> {
    //     let mut doc = Doc { files: Vec::default() };
    //     for file in files {
    //         let log = Log::open(Some(file))?;
    //         doc.push(log);
    //     }
    //     Ok(doc)
    // }

    pub fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = LogLine> + '_ {
        MergedLogsIterator::new(self)
    }
}

#[cfg(test)]
mod merged_logs_iterator_tests {

    use indexed_file::{files::{CursorLogFile, CursorUtil, CachedStreamReader, LogBase}, Log, indexer::LineIndexer};
    use super::MergedLogs;

    #[test]
    fn test_doc_basic() {
        let lines = 30000;
        let mut doc = MergedLogs::new();
        let buff = CursorLogFile::from_vec((0..lines).collect()).unwrap();
        doc.push_logbase(buff);

        // for line in doc.iter_lines() {
        //     print!(">>> {line}");
        // }
        // println!(); // flush

        assert_eq!(doc.iter_lines().count(), lines);
    }

    #[test]
    fn test_doc_merge() {
        let lines = 10;
        let mut doc = MergedLogs::new();

        let odds = (0..lines/2).map(|x| x * 2 + 1).collect();
        let odds = CursorLogFile::from_vec(odds).unwrap();
        doc.push_logbase(odds);

        let evens = (0..lines/2).map(|x| x * 2).collect();
        let evens = CursorLogFile::from_vec(evens).unwrap();
        doc.push_logbase(evens);

        let mut it = doc.iter_lines();
        let mut prev = it.next().unwrap();

        print!(">>> {prev}");
        for line in it {
            print!(">>> {prev} {line}");
            assert!(prev <= line);
            prev = line;
        }
        println!(); // flush

        assert_eq!(doc.iter_lines().count(), lines);
    }

    #[test]
    fn test_doc_merge_reverse() {
        let lines = 10;
        let mut doc = MergedLogs::new();

        let odds = (0..lines/2).map(|x| x * 2 + 1).collect();
        let odds = CursorLogFile::from_vec(odds).unwrap();
        doc.push_logbase(odds);

        let evens = (0..lines/2).map(|x| x * 2).collect();
        let evens = CursorLogFile::from_vec(evens).unwrap();
        doc.push_logbase(evens);

        let mut it = doc.iter_lines().rev();
        let mut prev = it.next().unwrap();

        print!(">>> {prev}");
        for line in it {
            print!(">>> {prev} {line}");
            assert!(prev >= line);
            prev = line;
        }
        println!(); // flush

        assert_eq!(doc.iter_lines().rev().count(), lines);
    }

    #[test]
    fn test_stream_reverse() {
        // FIXME: Test is failing on streams.  We can only iterate first element via MergedLogs.
        let lines = 10;
        let mut doc = MergedLogs::new();

        let nums = (0..lines).collect();
        let nums = CursorLogFile::from_vec(nums).unwrap();
        let nums = CachedStreamReader::from_reader(nums).unwrap();
        doc.push_logbase(nums);

        let mut it = doc.iter_lines().rev();
        let mut prev = it.next().unwrap();

        let mut count = 1;
        print!(">>> {prev}");
        for line in it {
            print!(">>> {prev} {line}");
            assert!(prev >= line);
            prev = line;
            count += 1;
        }
        println!(); // flush

        assert_eq!(doc.iter_lines().rev().count(), lines);
        assert_eq!(count, lines);
    }


    #[test]
    fn test_stream_reverse_inner() {
        // FIXME: Test is failing on streams.  We can only iterate first element via MergedLogs.
        let lines = 10;

        let nums = (0..lines).collect();
        let nums = CursorLogFile::from_vec(nums).unwrap();
        let nums = CachedStreamReader::from_reader(nums).unwrap();
        let mut log = Log::new(LineIndexer::new(nums.to_src()));

        let mut it = log.iter_lines().rev();
        let mut prev = it.next().unwrap();

        let mut count = 1;
        print!(">>> {prev}");
        for line in it {
            print!(">>> {prev} {line}");
            assert!(prev >= line);
            prev = line;
            count += 1;
        }
        println!(); // flush

        assert_eq!(log.iter_lines().rev().count(), lines);
        assert_eq!(count, lines);
    }
}