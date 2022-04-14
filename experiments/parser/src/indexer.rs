// Structs to index a file

use std::path::PathBuf;

use std::fs::File;
use std::io::{BufRead, BufReader};
// use std::collections::{HashMap};

use lazy_static::lazy_static;
use regex::Regex;
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
                }
            }
            _ => {
                if inword {
                    index.add_word(mmap[start..pos].to_vec(), cnt as usize);
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

    println!("Indexed tokens: {}",index.words.len());
    // index.split_numbers();

    println!("Total lines are: {}",cnt);
    println!("Total words are: {}",words);
    println!("Total bytes are: {}",bytes);
    println!("Indexed words: {}",index.words.len());
    println!("Indexed numbers: {}",index.numbers.len());
}