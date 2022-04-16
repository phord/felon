// Structs to index a file

use std::path::PathBuf;

use std::fs::File;
use fnv::FnvHashMap;
use std::collections::VecDeque;

pub struct Index {
    pub words: FnvHashMap<Vec<u8>, Vec<usize>>,
    pub numbers: FnvHashMap<u64, Vec<usize>>,
    // TODO: timestamps: FnvHashMap<u64, Vec<usize>>,
    // TODO: wordtree: Trie<>,  // a trie of words and all sub-words
}

impl Index {
    fn new() -> Index {
        Index {
            words: FnvHashMap::default(),
            numbers: FnvHashMap::default(),
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

    fn merge(&mut self, other: Index, line_start: usize) {
        // println!("Merging {} words into {} existing at line {}", other.words.len(), self.words.len(), line_start);
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
    }


    fn add_number(&mut self, number: u64, line: usize) {
        let lines = self.numbers.entry(number).or_insert(Vec::new());
        lines.push(line);
    }


    // fn search(&self, word: &str) -> Vec<usize> {
    //     let word = word.to_lowercase();
    //     let word = word.trim();
    //     if word.is_empty() {
    //         return Vec::new();
    //     }
    //     let word = word.to_string();
    //     match self.words.get(&word) {
    //         Some(words) => words.clone(),
    //         None => Vec::new(),
    //     }
    // }
}

use mapr::MmapOptions;

// Read part of the file and count the words/lines/characters
fn parse(data: &[u8]) -> (usize, Index) {

    let bytes = data.len();
    let mut cnt  = 0;
    // let mut words = 0;
    let mut index = Index::new();
    let mut start = 0;

    let mut inword = false;
    let mut inhexnum = false;
    let mut indecnum = false;
    let mut num:u64 = 0;
    let mut hexnum:u64 = 0;
    let mut pos = 0;
    loop {  // for pos in 0..bytes { //for c in mmap.as_ref() {
        if pos >= bytes {
            break;
        }
        let c = data[pos];
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
                            hexnum = hexnum * 16 + (c - b'0') as u64;
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
                        index.add_number(num, cnt);
                    } else if inhexnum {
                        index.add_number(hexnum, cnt);
                    } else {
                        index.add_word(&data[start..pos], cnt);
                    }
                    inword = false;
                }
                if c == b'\n' {
                    cnt += 1;
                    pos += 40;   // skip timestamp on next line
                }
            }
        }
        pos += 1;
    }
    (cnt, index)
}

use crossbeam::scope;
use crossbeam_channel::{bounded, unbounded};

pub fn index_file(input_file: Option<PathBuf>) -> Index{
    let input = if let Some(input_file) = input_file {
        // Must have a filename as input.
        let file = File::open(input_file).expect("Could not open file.");
        Some(file)
    } else {
        // Print error.
        eprintln!("Expected '<input>' or input over stdin.");
        ::std::process::exit(1);
    };

    let mmap = unsafe { MmapOptions::new().map(&input.unwrap()) };
    let mmap = mmap.expect("Could not mmap file.");
    let bytes = mmap.len();
    let chunk_size = 1024 * 1024 * 32;

    let mut pos = 0;

    struct ThreadData {
        start: usize,
        end: usize,
        lines: usize,
        index: Index,
    }

    let mut index = Index::new();
    let mut total_lines = 0;

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
            let mut line_offset = 0;
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
                        pos = data.end;
                        index.merge(data.index, line_offset);
                        line_offset += data.lines;
                        continue;
                    } else {
                        break; //exit held-poller loop; wait for new data
                    }
                }
            }
            mtx.send((line_offset, index)).unwrap();
        });

        // get indexes in chunks in threads
        while pos < bytes {
            let mut end = pos + chunk_size;
            if end > bytes {
                end = bytes;
            } else {
                // It would be nice to do this in parser, but we need an answer for the next thread or we can't proceed.
                while end < bytes && mmap[end] != b'\n' {
                    end += 1;
                }
            }

            // Count parser threads
            sender.send(true).unwrap();

            // Send the buffer to the parsers
            let buffer = &mmap[pos..end];

            let tx = tx.clone();
            let receiver = receiver.clone();
            let start = pos;
            scope.spawn(move |_| {
                let (lines, index) = parse(&buffer);
                let result = ThreadData {start,end,lines,index,};
                tx.send(result).unwrap();
                receiver.recv().unwrap();
            });
            pos = end;
        }

        // We don't need our own handle for this channel
        drop(tx);

        // Wait for results
        (total_lines, index) = mrx.iter().next().unwrap();
    }).unwrap();
    println!("Lines {}", total_lines);
    return index;
}