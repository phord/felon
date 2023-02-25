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
    line_offsets: Vec<usize>,
}

impl EventualIndex {
    fn new() -> EventualIndex {
        EventualIndex {
            indexes: Vec::new(),
            line_offsets: Vec::new(),
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

        let mut prev = self.indexes[0].start;
        // TODO: self.line_offsets is duplicate info; better to move from indexes or to always lookup from indexes
        for index in self.indexes.iter() {
            assert!(index.start == prev);
            prev = index.end;
            self.line_offsets.extend(index.iter());
        }
        // FIXME: Add index for end-of-file if not already present
        // e.g. if self.line_offsets.last() != self.indexes.last().end { self.line_offsets.push(self.indexes.last().end); }
    }

    fn line_offset(&self, line_number: usize) -> Option<usize> {
        if line_number >= self.line_offsets.len() {
            return None;
        }
        Some(self.line_offsets[line_number])
    }

    fn bytes(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| std::cmp::max(v.bytes(), a))
    }

    fn lines(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.lines())
    }

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
        let starts = std::iter::once(&0usize).chain(self.index.line_offsets.iter());
        let ends = self.index.line_offsets.iter();
        let line_range = starts.zip(ends);
        line_range
    }

    pub fn iter_lines(&self) -> impl Iterator<Item = (&str, usize, usize)> + '_ {
        self.iter_offsets().map(|(&start, &end)| -> (&str, usize, usize) {(self.readline_fixed(start, end).unwrap_or(""), start, end)})
    }

}
