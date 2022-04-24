// Structs to index a file

use std::path::PathBuf;

use std::fs::File;
use std::fmt;
use fnv::FnvHashMap;
use std::collections::BTreeSet;
use mapr::{MmapOptions, Mmap};
use crossbeam::scope;
use crossbeam_channel::{bounded, unbounded};

struct Index {
    words: FnvHashMap<Vec<u8>, Vec<usize>>,
    numbers: FnvHashMap<u64, Vec<usize>>,
    line_offsets: Vec<usize>,
    // TODO: timestamps: FnvHashMap<u64, Vec<usize>>,
    // TODO: wordtree: Trie<>,  // a trie of words and all sub-words
}

impl Index {
    fn new() -> Index {
        Index {
            words: FnvHashMap::default(),
            numbers: FnvHashMap::default(),
            line_offsets: Vec::new(),
        }
    }

    fn add_word(&mut self, word: &[u8], line: usize) {
        // let word = word.to_lowercase();
        // let word = word.trim();
        // if word.is_empty() {
        //     return;
        // }
        let lines = self.words.entry(word.to_vec()).or_insert(Vec::new());
        lines.push(line);
    }

    fn add_number(&mut self, number: u64, line: usize) {
        let lines = self.numbers.entry(number).or_insert(Vec::new());
        lines.push(line);
    }

    fn bytes(&self) -> usize {
        *self.line_offsets.last().unwrap_or(&0)
    }

    fn lines(&self) -> usize {
        self.line_offsets.len()
    }

    fn search_word(&self, word: &Vec<u8>) -> Option<BTreeSet<usize>> {
        match self.words.get(word) {
            Some(lines) => Some(BTreeSet::from_iter(lines.iter().cloned())),
            None => None,
        }
    }

    // TODO: search_line
    // Parse line into another index
    // Match index against self to find matching lines

    // Accumulate the map of words and numbers from the slice of lines
    // Parse buffer starting at `offset` and stopping at the first eol after end_target
    // Skip the first line unless offset is zero
    // size is the bytes we must process. After that is overlap with the next buffer.
    fn parse(&mut self, data: &[u8], offset: usize, size: usize) -> usize {
        let bytes = data.len();
        let has_final_eol = data.last().unwrap() == &b'\n';
        let mut cnt  = offset;
        // let mut words = 0;
        let mut start = 0;

        let mut inword = false;
        let mut inhexnum = false;
        let mut indecnum = false;
        let mut num:u64 = 0;
        let mut hexnum:u64 = 0;
        let mut pos = 0;
        let max_pos = if has_final_eol { bytes } else { bytes + 1 };

        // Skip the first line if offset is not zero
        if offset > 0 {
            for c in data {
                pos += 1;
                if c == &b'\n' {
                    cnt = offset + pos;
                    break;
                }
            }
        }

        loop {  // for pos in 0..bytes { //for c in mmap.as_ref() {
            if pos >= max_pos {
                break;
            }
            let c = if pos < bytes { data[pos] } else { b'\n' };
            match c {
                // All valid word or number characters
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' => {
                    if !inword {
                        inword = true;
                        // words += 1;
                        start = pos;
                        if c >= b'0' && c <= b'9' {
                            num = (c - b'0') as u64;
                            indecnum = true;
                            if c == b'0' {
                                inhexnum = true;
                                hexnum = 0;
                            }
                        }
                    } else {
                        if inhexnum {
                            if pos == start+1 {
                                inhexnum = c == b'x';
                            } else if !((c >= b'0' && c <= b'9') || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')) {
                                inhexnum = false;
                            } else if hexnum >= 1u64 << 61 {
                                // println!("Hex number too big @{cnt}: '{}' ==> {hexnum}", String::from_utf8((&data[start..pos]).to_vec()).unwrap());
                                inhexnum = false;
                            } else {
                                let nybble = if c <= b'9' {c - b'0'} else if c <= b'F' {10 + c - b'A'} else {10 + c - b'a'};
                                hexnum = hexnum * 16u64 + nybble as u64;
                            }
                        }
                        if indecnum {
                            if !(c >= b'0' && c <= b'9') {
                                indecnum = false;
                            } else if num >= (1u64 << 63) / 5 + 1 {
                                // println!("Decimal number too big @{cnt}: '{}' ==> {num}", String::from_utf8((&data[start..pos]).to_vec()).unwrap());
                                indecnum = false;
                            } else {
                                num = num * 10 + (c - b'0') as u64;
                            }
                        }
                    }
                }
                // All other characters (whitespace, punctuation)
                _ => {
                    if inword {
                        if indecnum {
                            self.add_number(num, cnt);
                        } else if inhexnum {
                            self.add_number(hexnum, cnt);
                        } else {
                            self.add_word(&data[start..pos], cnt);
                        }
                        inword = false;
                        inhexnum = false;
                        indecnum = false;
                    }
                    if c == b'\n' {
                        cnt = offset + pos + 1;
                        self.line_offsets.push(offset + pos + 1);
                        if pos >= size {
                            break;
                        }
                        pos += 40;   // skip timestamp on next line
                    }
                }
            }
            pos += 1;
        }
        cnt
    }
}

use std::cell::Cell;
use std::rc::Rc;

struct EventualIndex {
    indexes: Vec<Index>,
    words: Cell<FnvHashMap<Vec<u8>, Rc<BTreeSet<usize>>> >,
    numbers: FnvHashMap<u64, BTreeSet<usize>>,
    line_offsets: Vec<usize>,
}

impl EventualIndex {
    fn new() -> EventualIndex {
        EventualIndex {
            indexes: Vec::new(),
            words: Cell::new(FnvHashMap::default()),
            numbers: FnvHashMap::default(),
            line_offsets: Vec::new(),
        }
    }

    fn merge(&mut self, other: Index) {
        self.indexes.push(other);
        // if self.indexes.len() <= 1 {
        //     return;
        // }

        // I should be able to do this in, but I can't figure out how to do it
        // let &other = &self.indexes.last().unwrap();
        // self.indexes[0].line_offsets.extend_from_slice(&other.line_offsets);
    }

    fn finalize(&mut self) {
        self.indexes.sort_by_key(|index| *index.line_offsets.get(0).unwrap_or(&(1usize << 20)));

        // TODO: self.line_offsets is duplicate info; better to move from indexes or to always lookup from indexes
        for index in self.indexes.iter() {
            self.line_offsets.extend_from_slice(&index.line_offsets);
        }
    }

    fn line_number(&self, offset: usize) -> Option<usize> {
        // TODO: memoize or hashmap this
        let mut lo = 0;
        let mut hi = self.line_offsets.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.line_offsets[mid] == offset {
                return Some(mid);
            } else if self.line_offsets[mid] > offset {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        if self.line_offsets[lo] == offset {
            return Some(lo)
        } else {
            None
        }
    }

    fn line_offset(&self, line_number: usize) -> Option<usize> {
        if line_number >= self.line_offsets.len() {
            return None;
        }
        Some(self.line_offsets[line_number])
    }

    // Memoize a set of lines for a word and return a reference
    fn search_word<'a>(&'a self, word: &'a str) -> Rc<BTreeSet<usize>> {
        let word = word.as_bytes().to_vec();
        let mut words = self.words.take();

        let lines = if let Some(result) = words.get(&word) {
            result.clone()
        } else {
            println!("Searching for {}", String::from_utf8(word.clone()).unwrap());
            let mut result = BTreeSet::new();
            for index in &self.indexes {
                // TODO: Move lines out of original map?
                if let Some(lines) = &index.search_word(&word) {
                    result.extend(lines);
                }
            }
            let result = Rc::new(result);
            words.insert(word.clone(), result.clone());
            result
        };
        self.words.set(words);
        lines
}

    fn bytes(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| std::cmp::max(*v.line_offsets.last().or(Some(&0)).unwrap(), a))
    }

    fn lines(&self) -> usize {
        self.indexes.iter().fold(0, |a, v| a + v.lines())
    }

}


pub struct LogFile {
    // pub file_path: PathBuf,
    mmap: Mmap,
    index: EventualIndex,
}

impl fmt::Debug for LogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogFile")
         .field("bytes", &self.index.bytes())
        //  .field("words", &self.index.words.len())
        //  .field("numbers", &self.index.numbers.len())
         .field("lines", &self.index.lines())
         .finish()
    }
}

use std::io::{Error, ErrorKind};
impl LogFile {

    pub fn open(input_file: Option<PathBuf>) -> std::io::Result<LogFile> {

        let file = if let Some(file_path) = input_file {
            // Must have a filename as input.
            let file = File::open(file_path)?;
            Some(file)
        } else {

            // Print error.
            eprintln!("Expected '<input>' or input over stdin.");
            return Err(Error::new(ErrorKind::Other, "Expected a filename or stdin"));
        };

        let mmap = unsafe { MmapOptions::new().map(&file.unwrap()) };
        let mmap = mmap.expect("Could not mmap file.");

        let file = LogFile {
            // file_path: input_file.unwrap(),
            mmap,
            index: EventualIndex::new(),
        };

        Ok(file)
    }

    pub fn new(input_file: Option<PathBuf>) -> std::io::Result<LogFile> {
        let chunk_size = 1024 * 1024 * 1;
        let max_line_length: usize = 64 * 1024;

        let mut file = LogFile::open(input_file)?;
        file.index_file(chunk_size, max_line_length);
        Ok(file)
    }

    // FIXME: Is there a way to mark this for tests only?
    pub fn test_new(input_file: Option<PathBuf>, chunk_size: usize, max_line_length: usize) -> std::io::Result<LogFile> {
        let mut file = LogFile::open(input_file)?;
        file.index_file(chunk_size, max_line_length);
        Ok(file)
    }

    fn index_file(&mut self, chunk_size: usize, max_line_length: usize) {

        let bytes = self.mmap.len();
        let mut pos = 0;

        scope(|scope| {
            let (tx, rx):(crossbeam_channel::Sender<Index>, crossbeam_channel::Receiver<_>) = unbounded();
            // Limit threadpool of parsers by relying on sender queue length
            let (sender, receiver) = bounded(6); // inexplicably, 6 threads is ideal according to empirical evidence on my 8-core machine

            // get indexes in chunks in threads
            while pos < bytes {
                let end = std::cmp::min(pos + chunk_size, bytes);
                let overflow = std::cmp::min(bytes, end + max_line_length);

                // Count parser threads
                sender.send(true).unwrap();

                // Send the buffer to the parsers
                let buffer = &self.mmap[pos..overflow];

                let tx = tx.clone();
                let receiver = receiver.clone();
                let start = pos;
                scope.spawn(move |_| {
                    let mut index = Index::new();
                    index.parse(&buffer, start, end - start);
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

    pub fn search_word<'a>(&'a self, word: &'a str) -> Rc<BTreeSet<usize>> {
        return self.index.search_word(word);
    }

    pub fn readline_at(&self, offset: usize) -> Option<&str> {
        if let Some(line_number) = self.index.line_number(offset) {
            self.readline(line_number)
        } else {
            None
        }
    }

    pub fn readline(&self, line_number: usize) -> Option<&str> {
        let start = self.index.line_offset(line_number);
        let end = self.index.line_offset(line_number + 1);
        if let (Some(start), Some(end)) = (start, end) {
            assert!(end > start);
            // FIXME: Handle unwrap error
            Some(std::str::from_utf8(&self.mmap[start..end-1]).unwrap())
        } else {
            None
        }
    }
}