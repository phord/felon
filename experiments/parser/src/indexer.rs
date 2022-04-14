// Structs to index a file

use std::path::PathBuf;

use std::fs::File;
use fnv::FnvHashMap;

struct Index {
    words: FnvHashMap<Vec<u8>, Vec<usize>>,
    numbers: FnvHashMap<u64, Vec<usize>>,
}

impl Index {
    fn new() -> Index {
        Index {
            words: FnvHashMap::default(),
            numbers: FnvHashMap::default(),
        }
    }

    fn add_word(&mut self, word: Vec<u8>, line: usize) {
        // let word = word.to_lowercase();
        // let word = word.trim();
        // if word.is_empty() {
        //     return;
        // }
        let lines = self.words.entry(word).or_insert(Vec::new());
        lines.push(line);
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

// Read the file and count the words/lines/characters
pub fn run(input_file: Option<PathBuf>,) {
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

    let mut cnt  = 0;
    let bytes = mmap.len();
    let mut words = 0;
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
        let c = mmap[pos];
        match c {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' => {
                if !inword {
                    inword = true;
                    words += 1;
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

            _ => {
                if inword {
                    if indecnum {
                        index.add_number(num, cnt);
                    } else if inhexnum {
                        index.add_number(hexnum, cnt);
                    } else {
                        index.add_word(mmap[start..pos].to_vec(), cnt);
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

    println!("Total lines are: {}",cnt);
    println!("Total words are: {}",words);
    println!("Total bytes are: {}",bytes);
    println!("Indexed words: {}",index.words.len());
    println!("Indexed numbers: {}",index.numbers.len());
}