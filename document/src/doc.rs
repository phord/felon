// A collection of log lines from multiple log files.
// Lines can be blended by merging (sorted) or by concatenation.

use indexed_file::Log;
use indexed_file::files::{CursorLogFile, CursorUtil, LogFile};
use indexed_file::files::LogBase;
use indexed_file::files::LogSource;
use indexed_file::indexer::LineIndexer;

/* Thinking:
   Hold a vec of files.
   Each line is indexed by doc-offset.

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
pub struct Doc {
    files: Vec<Log>
}

type Iter<'a> = Box<dyn DoubleEndedIterator<Item = (String, usize)> + 'a>;

struct LogIter<'a> {
    next: Option<String>,
    prev: Option<String>,
    iter: Iter<'a>,
}

impl<'a> LogIter<'a> {
    fn new(file: &'a mut Log) -> Self {
        Self {
            iter: Box::new(file.iter_lines()),
            next: None,
            prev: None,
        }
    }

    // Return ref to next string unless EOF, else prev string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn peek_next(&mut self) -> &Option<String> {
        if self.next.is_some() || self.advance() {
            &self.next
        } else {
            &self.prev
        }
    }

    // Return ref to prev string unless EOF, else next string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn peek_prev(&mut self) -> &Option<String> {
        if self.prev.is_some() || self.advance_back() {
            &self.prev
        } else {
            &self.next
        }
    }

    fn advance(&mut self) -> bool {
        // Pre-load the next line for peek
        self.next =
            if let Some((line, _offset)) = self.iter.next() {
                // FIXME: Return offset to construct a Cursor with
                Some(line)
            } else {
                None
            };
        self.next.is_some()
    }

    fn advance_back(&mut self) -> bool {
        // Pre-load the prev line for peek
        self.prev =
            if let Some((line, _offset)) = self.iter.next_back() {
                // FIXME: Return offset to construct a Cursor with
                Some(line)
            } else {
                None
            };
        self.prev.is_some()
    }

    // Return next string unless EOF, else prev string
    // Assumes that prev and next are approaching each other in this DoubleEndedIterator
    fn take_next(&mut self) -> Option<String> {
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
    fn take_prev(&mut self) -> Option<String> {
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

// A semi-sorted iterator over Doc
pub(crate) struct DocIterator<'a> {
    iters: Vec<LogIter<'a>>,
}

impl<'a> DocIterator<'a> {
    pub(crate) fn new(doc: &'a mut Doc) -> Self {
        Self {
            iters: doc.files
                    .iter_mut()
                    .map(|log| LogIter::new(log))
                    .collect(),
        }
    }
}

impl<'a> Iterator for DocIterator<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, _line)) = self.iters
            .iter_mut()
            .map(|iter| iter.peek_next())
            .enumerate()
            .filter(|(_, v)| v.is_some())
            .min_by(|(_, line0), (_, line1)| line0.cmp(line1)) {
                self.iters[i].take_next()
        } else {
            None
        }
    }
}

impl<'a> DoubleEndedIterator for DocIterator<'a> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some((i, _line)) = self.iters
            .iter_mut()
            .map(|iter| iter.peek_prev())
            .enumerate()
            .filter(|(_, v)| v.is_some())
            .max_by(|(_, line0), (_, line1)| line0.cmp(line1)) {
                self.iters[i].take_prev()
        } else {
            None
        }
    }
}


impl Doc {
    pub fn new() -> Self {
        Doc { files: Vec::default() }
    }

    pub fn push<L: LogBase + 'static>(&mut self, log: L) {
        self.files.push(LineIndexer::new(log.to_src()));
    }

    // pub fn new(files: Vec<PathBuf>) -> std::io::Result<Self> {
    //     let mut doc = Doc { files: Vec::default() };
    //     for file in files {
    //         let log = Log::open(Some(file))?;
    //         doc.files.push(log);
    //     }
    //     Ok(doc)
    // }

    pub fn iter_lines(&mut self) -> impl DoubleEndedIterator<Item = String> + '_ {
        DocIterator::new(self)
    }
}
#[test]
fn test_doc_basic() {
    let lines = 30000;
    let mut doc = Doc::new();
    let buff = CursorLogFile::from_vec((0..lines).into_iter().collect()).unwrap();
    doc.push(buff);

    // for line in doc.iter_lines() {
    //     print!(">>> {line}");
    // }
    // println!(); // flush

    // FIXME: We get one extra line at the end of the file because that's how indexer currently works.
    assert_eq!(doc.iter_lines().count(), lines + 1);
}

#[test]
fn test_doc_merge() {
    let lines = 10;
    let mut doc = Doc::new();

    let odds = (0..lines/2).into_iter().map(|x| x * 2 + 1).collect();
    let odds = CursorLogFile::from_vec(odds).unwrap();
    doc.push(odds);

    let evens = (0..lines/2).into_iter().map(|x| x * 2).collect();
    let evens = CursorLogFile::from_vec(evens).unwrap();
    doc.push(evens);

    let mut it = doc.iter_lines();
    let mut prev = it.next().unwrap();

    print!(">>> {prev}");
    for line in it {
        if line.is_empty() {
            // Empty lines at end of files disturb our sense of order.  Ignore them.
            continue
        }
        print!(">>> {prev} {line}");
        assert!(prev <= line);
        prev = line;
    }
    println!(); // flush

    // FIXME: We get one extra line at the end of each file because that's how indexer currently works.
    assert_eq!(doc.iter_lines().count(), lines + 2);
}