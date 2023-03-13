// Structs to index lines in a text file
// TODO: Cleanup - This is a clone of indexer (LogFile) that doesn't parse out words and numbers.  It only parses lines.
//       Needs to be allowed to run in the background better, in a way that Rust can accept.

#[cfg(test)]
use std::path::PathBuf;

use std::fmt;
use crossbeam::scope;
use crossbeam_channel::{bounded, unbounded};
use crate::log_file::{LogFile, LogFileTrait};
use crate::index::Index;


struct EventualIndex {
    indexes: Vec<Index>,
}

impl EventualIndex {
    fn new() -> EventualIndex {
        EventualIndex {
            indexes: Vec::new(),
        }
    }

    fn merge(&mut self, other: Index) {
        // merge lazily
        self.indexes.push(other);
    }

    fn finalize(&mut self) {
        if self.indexes.is_empty() {
            return;
        }

        self.indexes.sort_by_key(|index| index.start);

        // let mut prev = self.indexes[0].start;
        // for index in self.indexes.iter() {
        //     assert_eq!(index.start, prev);
        //     prev = index.end;
        // }

        // FIXME: Add index for end-of-file if not already present
        // e.g. if self.line_offsets.last() != self.indexes.last().end { self.line_offsets.push(self.indexes.last().end); }
    }

    fn line_offset(&self, line_number: usize) -> Option<usize> {
        if line_number >= self.lines() {
            return None;
        }
        self.iter().skip(line_number).cloned().next()
    }

    pub fn iter(self: &Self) -> impl DoubleEndedIterator<Item = &usize> {
        self.indexes.iter().flat_map(|x| x.iter())
    }

    fn bytes(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.bytes())
    }

    fn lines(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.lines())
    }
}

#[derive(Debug, Copy, Clone)]
enum FindIndex {
    // Unknown
    None,

    // Line is index to start of file
    StartOfFile,

    // Line is in this index at this offset
    IndexOffset(usize, usize),

    // Position is not indexed; need to index region from given `start` to `end`
    Missing(usize, usize),

    // Position is not indexed; unknown gap size at end of index needs to be loaded; arg is first unindexed byte
    MissingUnbounded(usize),
}

// Holds reference information into an EventualIndex so lines can be navigated before the whole index
// is known.
#[derive(Debug,Clone)]
struct LineCursor {
    // Expected index and position to find the line
    index_hint: FindIndex,
}

impl LineCursor {
    fn new() -> LineCursor {
        Self {
            index_hint: FindIndex::StartOfFile,
        }
    }
}


// Tests for EventualIndex
#[cfg(test)]
mod tests {
    use crate::index::Index;
    use crate::line_indexer::EventualIndex;
    use crate::line_indexer::LineCursor;

    use super::FindIndex;
    static STRIDE: usize = 2;
    static DATA: &str = "a\na\na\na\na\n";
    static END: usize = DATA.len();
    static OFFSETS:[usize; 5] = [2,4,6,8,10];

    // // Verify index.line_offsets match expected set only in the range [start, end]
    // fn check_partial(index: &Index, start:usize, end: usize) {
    //     let offsets: Vec<usize> =
    //         OFFSETS
    //             .iter()
    //             .filter(|x| **x >= start && **x <= end)
    //             .cloned()
    //             .collect();
    //     assert_eq!(index.iter().cloned().collect::<Vec<usize>>(), offsets);
    // }

    fn get_index(offset: usize) -> Index {
        let mut index = Index::new();
        index.parse(DATA.as_bytes(), offset);
        index
    }

    fn get_eventual_index(size: usize) -> EventualIndex {
        let mut index = EventualIndex::new();
        while index.bytes() < size {
            let s = index.bytes();
            println!("Size {s}");
            index.merge(get_index(index.bytes()));
        }
        index.finalize();
        index
    }

    fn get_partial_eventual_index(start: usize, size: usize) -> EventualIndex {
        let mut index = EventualIndex::new();
        while index.bytes() < size {
            let s = index.bytes();
            println!("Size {s}");
            index.merge(get_index(start + index.bytes()));
        }
        index.finalize();
        index
    }

    #[test]
    fn test_eventual_index_basic() {
        let index = get_eventual_index(100);
        assert_eq!(index.bytes(), 100);
        assert_eq!(index.lines(), 50);
    }

    #[test]
    fn test_cursor_start() {
        let index = get_eventual_index(100);
        let cursor = index.find_index(0);
        dbg!(cursor);
        match cursor {
            FindIndex::StartOfFile => {},
            _ => {
                dbg!(cursor);
                panic!("Expected StartOfFile; got something else");
            }
        }
    }


    #[test]
    fn test_cursor_mid_start() {
        let index = get_partial_eventual_index(50, 100);
        let cursor = index.find_index(50);
        match cursor {
            FindIndex::IndexOffset(0, 0) => {},
            _ => panic!("Expected Index(0,0); got something else: {:?}", cursor),
        }
        let fault = index.find_index(10);
        match fault {
            FindIndex::Missing(0, 50) => {},
            _ => panic!("Expected Missing(0,50); got something else: {:?}", fault),
        }
    }

    #[test]
    fn test_cursor_last() {
        let index = get_eventual_index(100);
        let cursor = index.find_index(index.bytes()-1);
        match cursor {
            FindIndex::IndexOffset(_,_) => {},
            _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
        }
        let fault = index.find_index(index.bytes());
        match fault {
            FindIndex::MissingUnbounded(_) => {},
            _ => panic!("Expected MissingUnbounded; got something else: {:?}", fault),
        }
    }

    #[test]
    fn test_cursor_forward() {
        let index = get_eventual_index(100);
        let mut cursor = index.find_index(0);
        let mut count = 0;
        loop {
            // dbg!(&cursor);
            match cursor {
                FindIndex::IndexOffset(_,_) => {},
                FindIndex::StartOfFile => {},
                FindIndex::MissingUnbounded(_) => break,
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
            count += 1;
            println!("Line {}  Cursor: {} {}", count, index.start_of_line(cursor).unwrap(), index.end_of_line(cursor).unwrap());
            cursor = index.next_line_index(cursor);
        }
        assert_eq!(count, index.lines());
    }

    #[test]
    fn test_cursor_reverse() {
        let index = get_eventual_index(100);
        let mut cursor = index.find_index(99);
        let mut count = 0;
        loop {
            // dbg!(&cursor);
            count += 1;
            println!("Line {}  Cursor: {} {}", count, index.start_of_line(cursor).unwrap(), index.end_of_line(cursor).unwrap());
            match cursor {
                FindIndex::IndexOffset(_,_) => {},
                FindIndex::StartOfFile => break,
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
            cursor = index.prev_line_index(cursor);
        }
        assert_eq!(count, index.lines());
    }

    #[test]
    fn test_cursor_reverse_gap() {
        let index = get_partial_eventual_index(50, 100);
        let mut cursor = index.find_index(149);
        let mut count = 0;
        loop {
            dbg!(&cursor);
            match cursor {
                FindIndex::IndexOffset(_,_) => {},
                FindIndex::Missing(0,50) => break,
                _ => panic!("Expected IndexOffset; got something else: {:?}", cursor),
            }
            count += 1;
            cursor = index.prev_line_index(cursor);
        }
        assert_eq!(count, index.lines());
    }
}

// Cursor functions for EventualIndex
impl EventualIndex {

    // Identify the gap before a given index position and return a Missing() hint to include it.
    fn gap_at(&self, pos: usize) -> FindIndex {
        assert!(pos <= self.indexes.len());
        if pos == 0 {
            // gap is at start of file
            let next = self.indexes[pos].start;
            FindIndex::Missing(0, next)
        } else {
            let prev = self.indexes[pos-1].end;
            if pos == self.indexes.len() {
                // gap is at end of file; return unbounded range
                FindIndex::MissingUnbounded(prev)
            } else {
                // There's a gap between two indexes; bracket result by their [end, start)
                let next = self.indexes[pos].start;
                FindIndex::Missing(prev, next)
            }
        }
    }

    // Find index to EOL that contains a given offset or the gap that needs to be loaded to have it
    fn find_index(&self, offset: usize) -> FindIndex {
        match self.indexes.binary_search_by(|i| i.contains_offset(&offset)) {
            Ok(found) => {
                let i = &self.indexes[found];
                let line = i.find(offset).unwrap();
                if line == 0 && i.start == 0 {
                    FindIndex::StartOfFile
                } else if line < i.len() {
                    FindIndex::IndexOffset(found, line)
                } else {
                    self.next_line_index(FindIndex::IndexOffset(found, line-1))
                }
            },
            Err(after) => {
                // No index holds our offset; it needs to be loaded
                self.gap_at(after)
            }
        }
    }

    // True if there is no gap between given index and the next one
    fn next_is_contiguous(&self, pos: usize) -> bool {
        assert!(pos < self.indexes.len());
        pos + 1 < self.indexes.len() && {
            let i = &self.indexes[pos];
            let j = &self.indexes[pos + 1];
            assert!(j.start >= i.end);
            j.start == i.end
        }
    }

    // Find index to next EOL after given index
    fn next_line_index(&self, find: FindIndex) -> FindIndex {
        match find {
            FindIndex::StartOfFile => {
                    // TODO: Get rid of this weirdo
                    if self.indexes.is_empty() {
                        self.gap_at(0)
                    } else {
                        FindIndex::IndexOffset(0, 1)
                    }
                },
            FindIndex::IndexOffset(found, line) => {
                    // next line is in in the same index
                    assert!(found < self.indexes.len());
                    let i = &self.indexes[found];
                    if line + 1 < i.len() {
                        FindIndex::IndexOffset(found, line + 1)
                    } else if self.next_is_contiguous(found) {
                        FindIndex::IndexOffset(found+1, 0)
                    } else {
                        self.gap_at(found + 1)
                    }
                },
            _ => find,
        }
    }

    // Find index to prev EOL before given index
    fn prev_line_index(&self, find: FindIndex) -> FindIndex {
        if let FindIndex::IndexOffset(found, line) = find {
            // next line is in the same index
            assert!(found < self.indexes.len());
            let i = &self.indexes[found];
            if i.start == 0 && line == 1 {
                FindIndex::StartOfFile   // TODO: Weird special case for first line in file.
            } else if line > 0 {
                FindIndex::IndexOffset(found, line - 1)
            } else if found > 0 && self.next_is_contiguous(found - 1) {
                let j = &self.indexes[found - 1];
                FindIndex::IndexOffset(found - 1, j.len() - 1)
            } else {
                self.gap_at(found)
            }
        } else {
            find
        }
    }

    // Return offset of start of indexed line, if known
    fn start_of_line(&self, find: FindIndex) -> Option<usize> {
        match find {
            FindIndex::StartOfFile => Some(0),

            FindIndex::IndexOffset(found, line) => {
                    // This line starts one byte after the previous one ends
                    let find = self.prev_line_index(find);
                    let prev_eol = self.end_of_line(find);
                    if let Some(eol) = prev_eol {
                        Some(eol + 1)
                    } else {
                        None
                    }
                },
            _ => None,
        }
    }

    // Return offset of end of indexed line, if known
    fn end_of_line(&self, find: FindIndex) -> Option<usize> {
        match find {
            FindIndex::StartOfFile => {
                if self.indexes.is_empty() {
                    None
                } else {
                    let i = &self.indexes[0];
                    Some(i.get(0))
                }
            },

            FindIndex::IndexOffset(found, line) => {
                    assert!(found < self.indexes.len());
                    let i = &self.indexes[found];
                    Some(i.get(line))
                },
            _ => None,
        }
    }

    // // Verify a cursor's index hint, fill it in, or leave it open if index doesn't exist yet
    // fn resolve_cursor(&self, c: LineCursor) -> LineCursor {
    //     let eol = c.offset + c.len;
    //     if let FindIndex::IndexOffset(ind, pos) = c.index_hint {
    //         if ind < self.indexes.len() {
    //             let index = &self.indexes[ind];
    //             if pos < index.len() && index.get(pos) == eol {
    //                 // Cursor is already resolved
    //                 return c;
    //             }
    //         }
    //     }

    //     // Find an existing index that holds our offset
    //     let end = self.find_index(c.offset);
    //     if end.is_some() {
    //         todo!("Find previous entry and calculate offset and length");
    //         return LineCursor {
    //             offset: c.offset,
    //             len: c.len,
    //             index_hint: Some((i, end)),
    //         };
    //     }

    //     // Didn't find our target.  Return empty-handed.
    //     return LineCursor {
    //         offset: c.offset,
    //         len: c.len,
    //         index_hint: None,
    //     };
    // }

    // // Find the index that ends at a given pos and return a cursor to its last line
    // fn find_cursor_at_end(&self, pos: usize) -> LineCursor {
    //     // Find an existing index that ends at our offset
    //     for (i, index) in self.indexes.iter().enumerate() {
    //         if pos + 1 == index.end {
    //             // FIXME: Don't allow empty indexes; always merge with neighbors or keep searching
    //             assert!(index.len() > 0);
    //             let iter = index.iter().rev();
    //             let end = iter.next().unwrap();
    //             let prev = iter.next().unwrap();
    //             return LineCursor {
    //                 offset: prev + 1,
    //                 len: end - prev,
    //                 index_hint: Some((i, index.len()-1)),
    //             };
    //         }
    //     }

    //     // Didn't find our target.  Return empty-handed.
    //     return LineCursor {
    //         offset: pos,
    //         len: 0,
    //         index_hint: None,
    //     };
    // }

    // // Find the index that begins at a given pos and return a cursor to that line
    // fn find_cursor_at_start(&self, pos: usize) -> LineCursor {
    //     // Find an existing index that starts at our offset
    //     for (i, index) in self.indexes.iter().enumerate() {
    //         if pos == index.start {
    //             // FIXME: Don't allow empty indexes; always merge with neighbors or keep searching
    //             assert!(index.len() > 0);
    //             return LineCursor {
    //                 offset: *index.iter().next().unwrap(),
    //                 index_hint: Some((i, 0)),
    //             };
    //         }
    //     }

    //     // Didn't find our target.  Return empty-handed.
    //     return LineCursor {
    //         offset: pos,
    //         index_hint: None,
    //     };
    // }

    // fn get_cursor(&self, ind: usize, pos: usize) -> LineCursor {
    //     assert!(ind < self.indexes.len());
    //     let index = &self.indexes[ind];
    //     assert!(pos < index.len());
    //     LineCursor {
    //         offset: index.get(pos),
    //         index_hint: Some((ind, pos)),
    //     }
    // }

    // // Expect: c is already resolved
    // // Returns cursor for previous line before c, if already indexed
    // fn prev_line(&self, c: LineCursor) -> LineCursor {
    //     assert!(c.index_hint.is_some());
    //     if let Some((ind, pos)) = c.index_hint {
    //         if pos > 0 {
    //             return self.get_cursor(ind, pos);
    //         } else {
    //             return self.find_cursor_at_end(c.offset - 1);
    //         }
    //     } else {
    //         // Didn't find our target.  Return empty-handed.
    //         return LineCursor {
    //             offset: c.offset,
    //             index_hint: None,
    //         };
    //     }
    // }

    // // Expect: c is already resolved
    // // Returns cursor for next line after c, if already indexed
    // fn next_line(&self, c: LineCursor) -> LineCursor {
    //     assert!(c.index_hint.is_some());
    //     if let Some((ind, pos)) = c.index_hint {
    //         assert!(ind < self.indexes.len());
    //         let index = &self.indexes[ind];
    //         if pos + 1 < index.len() {
    //             return self.get_cursor(ind, pos + 1);
    //         } else {
    //             let end = index.iter().rev().next().unwrap();
    //             return self.find_cursor_at_start(*end);
    //         }
    //     } else {
    //         // Didn't find our target.  Return empty-handed.
    //         return LineCursor {
    //             offset: c.offset,
    //             index_hint: None,
    //         };
    //     }
    // }
}

pub struct LogFileLines {
    // pub file_path: PathBuf,
    file: LogFile,
    index: EventualIndex,
}

impl fmt::Debug for LogFileLines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogFileLines")
         .field("bytes", &self.count_bytes())
         .field("lines", &self.count_lines())
         .finish()
    }
}

impl LogFileLines {

    pub fn new(file: LogFile) -> LogFileLines {
        let chunk_size = 1024 * 1024 * 1;

        let mut index = Self {
            file,
            index: EventualIndex::new(),
        };
        index.index_file(chunk_size);
        index
    }

    fn index_file(&mut self, chunk_size: usize) {

        let bytes = self.file.len();
        let mut pos = 0;

        // TODO: Since lazy merge is free, kick off the threads here and keep them running. Then any readers
        // can collect results and merge them to get completed progress in real-time. This also give us a
        // chance to add a stop-signal so we can exit early.

        // Finalize needs to adapt, and this loop needs to run in its own thread.
        // In the future this mechanism can serve to read like tail -f or to read from stdin.

        let (tx, rx):(crossbeam_channel::Sender<Index>, crossbeam_channel::Receiver<_>) = unbounded();
        // Limit threadpool of parsers by relying on sender queue length
        let (sender, receiver) = bounded(6); // inexplicably, 6 threads is ideal according to empirical evidence on my 8-core machine

        scope(|scope| {
            // get indexes in chunks in threads
            while pos < bytes {
                let end = std::cmp::min(pos + chunk_size, bytes);

                // Count parser threads
                sender.send(true).unwrap();

                // Send the buffer to the parsers
                let buffer = self.file.read(pos, end-pos).unwrap();

                let tx = tx.clone();
                let receiver = receiver.clone();
                let start = pos;
                scope.spawn(move |_| {
                    let mut index = Index::new();
                    index.parse(buffer, start);
                    tx.send(index).unwrap();
                    receiver.recv().unwrap();
                });
                pos = end;
            }

            // We don't need our own handle for this channel
            drop(tx);

            // Wait for results and merge them in
            while let Ok(index) = rx.recv() {
                self.index.merge(index);
            }
        }).unwrap();

        // Partially coalesce merged info
        self.index.finalize();
    }

    fn count_bytes(&self) -> usize {
        self.index.bytes()
    }

    pub fn count_lines(&self) -> usize {
        self.index.lines()
    }

    fn line_offset(&self, line_number: usize) -> Option<usize> {
        if line_number == 0 {
            Some(0)
        } else {
            self.index.line_offset(line_number - 1)
        }
    }

    pub fn readline(&self, line_number: usize) -> Option<&str> {
        let start = self.line_offset(line_number);
        let end = self.line_offset(line_number + 1);
        if let (Some(start), Some(end)) = (start, end) {
            self.readline_fixed(start, end)
        } else {
            None
        }
    }

    pub fn readline_fixed(&self, start: usize, end: usize) -> Option<&str> {
        if end < self.file.len() {
            assert!(end > start);
            // FIXME: Handle unwrap error
            // FIXME: Handle CR+LF endings
            Some(std::str::from_utf8(self.file.read(start, end - start - 1).unwrap()).unwrap())
        } else {
            None
        }
    }

    pub fn iter_offsets(&self) -> impl Iterator<Item = (&usize, &usize)> + '_ {
        let starts = std::iter::once(&0usize).chain(self.index.iter());
        let ends = self.index.iter();
        let line_range = starts.zip(ends);
        line_range
    }

    pub fn iter_lines(&self) -> impl Iterator<Item = (&str, usize, usize)> + '_ {
        self.iter_offsets().map(|(&start, &end)| -> (&str, usize, usize) {(self.readline_fixed(start, end).unwrap_or(""), start, end)})
    }

}
