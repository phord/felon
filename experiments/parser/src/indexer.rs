// Structs to index a file

use std::path::PathBuf;

use std::fs::File;
use std::fmt;
use fnv::FnvHashMap;
use std::collections::VecDeque;
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

    fn merge(&mut self, other: Index) {
        let line_start = self.line_offsets.len();
        for (word, l) in other.words {
            let lines = self.words.entry(word).or_insert(Vec::new());
            for line in l {
                lines.push(line + line_start);
            }
        }
        for (number, l) in other.numbers {
            let lines = self.numbers.entry(number).or_insert(Vec::new());
            for line in l {
                lines.push(line + line_start);
            }
        }
        // TODO: Use `append` for speed? Or use split_vectors?  skip_lists?
        self.line_offsets.extend_from_slice(&other.line_offsets);
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

    // FIXME: memoize this and return a reference
    fn search_word(&self, word: &str) -> Option<BTreeSet<usize>> {
        let word = word.trim();
        if word.is_empty() {
            return None;
        }
        let word = word.as_bytes().to_vec();
        match self.words.get(&word) {
            Some(lines) => Some(BTreeSet::from_iter(lines.iter().cloned())),
            None => None,
        }
    }

    // TODO: search_line
    // Parse line into another index
    // Match index against self to find matching lines

    // Accumulate the map of words and numbers from the slice of lines
    fn parse(&mut self, data: &[u8], offset: usize) -> usize {
        let bytes = data.len();
        let has_final_eol = data.last().unwrap() == &b'\n';
        let mut cnt  = 0;
        // let mut words = 0;
        let mut start = 0;

        let mut inword = false;
        let mut inhexnum = false;
        let mut indecnum = false;
        let mut num:u64 = 0;
        let mut hexnum:u64 = 0;
        let mut pos = 0;
        let max_pos = if has_final_eol { bytes } else { bytes + 1 };
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
                            if pos == bytes+1 && c == b'x' {
                                // inhexnum = true;
                            } else if !((c >= b'0' && c <= b'9') || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')) {
                                inhexnum = false;
                            } else {
                                let nybble = if c <= b'9' {c - b'0'} else if c <= b'F' {10 + c - b'A'} else {10 + c - b'a'};
                                hexnum = hexnum * 16 + nybble as u64;
                            }
                        }
                        if indecnum {
                            if !(c >= b'0' && c <= b'9') {
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
                    }
                    if c == b'\n' {
                        cnt += 1;
                        self.line_offsets.push(offset + std::cmp::max(pos + 1, bytes));
                        pos += 40;   // skip timestamp on next line
                    }
                }
            }
            pos += 1;
        }
        cnt
    }
}

pub struct LogFile {
    // pub file_path: PathBuf,
    mmap: Mmap,
    index: Index,
}

impl fmt::Debug for LogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogFile")
         .field("bytes", &self.index.bytes())
         .field("words", &self.index.words.len())
         .field("numbers", &self.index.numbers.len())
         .field("lines", &self.index.lines())
         .finish()
    }
}

impl LogFile {
    // FIXME: Return a Result<> to pass errors upstream
    pub fn new(input_file: Option<PathBuf>) -> LogFile {

        let file = if let Some(file_path) = input_file {
            // Must have a filename as input.
            let file = File::open(file_path).expect("Could not open file.");
            Some(file)
        } else {
            // Print error.
            eprintln!("Expected '<input>' or input over stdin.");
            ::std::process::exit(1);
        };

        let mmap = unsafe { MmapOptions::new().map(&file.unwrap()) };
        let mmap = mmap.expect("Could not mmap file.");

        let mut file = LogFile {
            // file_path: input_file.unwrap(),
            mmap,
            index: Index::new(),
        };

        file.index_file();
        file
    }

    fn index_file(&mut self) {

        let bytes = self.mmap.len();
        let chunk_size = 1024 * 1024 * 64;

        let mut pos = 0;

        struct ThreadData {
            start: usize,
            index: Index,
        }

        let mut index = Index::new();

        scope(|scope| {
            let (tx, rx) = unbounded();
            let (mtx, mrx) = unbounded();
            // Limit threadpool of parsers by relying on sender queue length
            let (sender, receiver) = bounded(6); // inexplicably, 6 threads is ideal according to empirical evidence on my 8-core machine

            // Thread to merge index results
            //   rx ---> [merged] ---> mtx ---> [index]
            scope.spawn(move |_| {
                let mut held: VecDeque<ThreadData> = VecDeque::new();
                let mut pos = 0;
                let mut index = Index::new();
                while let Ok(data) = rx.recv() {
                    held.push_front(data);
                    loop {
                        let mut data = None;
                        for i in 0..held.len() {
                            if held[i].start == pos {
                                data = held.remove(i);
                                break;
                            }
                        }
                        if let Some(data) = data {
                            index.merge(data.index);
                            pos = index.bytes();
                            continue;
                        } else {
                            break; //exit held-poller loop; wait for new data
                        }
                    }
                }
                assert!(held.is_empty());
                mtx.send(index).unwrap();
            });

            // get indexes in chunks in threads
            while pos < bytes {
                let mut end = pos + chunk_size;
                if end > bytes {
                    end = bytes;
                } else {
                    // It would be nice to do this in parser, but we need an answer for the next thread or we can't proceed.
                    while end < bytes && self.mmap[end] != b'\n' {
                        end += 1;
                    }
                    // Point past eol, if there is one
                    if end < bytes {
                        assert_eq!(self.mmap[end], b'\n');
                        end += 1;
                    }
                }

                // Count parser threads
                sender.send(true).unwrap();

                // Send the buffer to the parsers
                let buffer = &self.mmap[pos..end];

                let tx = tx.clone();
                let receiver = receiver.clone();
                let start = pos;
                scope.spawn(move |_| {
                    let mut index = Index::new();
                    index.parse(&buffer, start);
                    let result = ThreadData {start, index, };
                    tx.send(result).unwrap();
                    receiver.recv().unwrap();
                });
                pos = end;
            }

            // We don't need our own handle for this channel
            drop(tx);

            // Wait for results
            index = mrx.iter().next().unwrap();
        }).unwrap();
        self.index = index;
    }

    // FIXME: memoize this and return a reference
    pub fn search_word(&self, word: &str) -> Option<BTreeSet<usize>> {
        return self.index.search_word(word);
    }
}