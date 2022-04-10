// Structs to index a file

use std::path::PathBuf;

use std::fs::File;
use std::io::{BufRead, BufReader};
// use std::collections::{HashMap};

use lazy_static::lazy_static;
use regex::Regex;
use fnv::FnvHashMap;

struct Index {
    words: FnvHashMap<String, Vec<usize>>,
    numbers: FnvHashMap<u64, Vec<usize>>,
}

impl Index {
    fn new() -> Index {
        Index {
            words: FnvHashMap::default(),
            numbers: FnvHashMap::default(),
        }
    }

    fn add_word(&mut self, word: &str, line: usize) {
        // let word = word.to_lowercase();
        // let word = word.trim();
        // if word.is_empty() {
        //     return;
        // }
        let word = word.to_string();
        let lines = self.words.entry(word).or_insert(Vec::new());
        lines.push(line);
    }

    fn split_numbers(&mut self) {
        lazy_static! {
            static ref HEX_RE: Regex = Regex::new(r"^0x[[:xdigit:]]+$").unwrap();
            static ref DEC_RE: Regex = Regex::new(r"^\d+$").unwrap();
        }

        self.words.retain(|word, lines| {
            // Partition the words into numbers and non-numbers
            let num = if HEX_RE.is_match(&word) {
                let word = word.trim_start_matches("0x");
                Some(u64::from_str_radix(word, 16).expect(word)) // FIXME: handle error
            } else if DEC_RE.is_match(&word) {
                Some(u64::from_str_radix(word, 10).expect(word)) // FIXME: handle error
            } else {
                None
            };

            match num {
                Some(num) => {
                    self.numbers.entry(num)
                        .and_modify(|e| e.extend(lines.iter()))
                        .or_insert(lines.clone());
                    false
                }
                None => true
            }
        });
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

    let file = BufReader::new(input.unwrap());
    let mut cnt  = 0;
    let mut bytes = 0;
    let mut words = 0;
    let mut index = Index::new();

    for l in file.lines() {
        cnt = cnt + 1;
        let line: String = l.unwrap().clone();
        bytes += line.len() + 1;
        for w in line.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if !w.is_empty() {
                words += 1;
                index.add_word(w, cnt);
            }
        }
    }

    println!("Indexed tokens: {}",index.words.len());
    index.split_numbers();

    println!("Total lines are: {}",cnt);
    println!("Total words are: {}",words);
    println!("Total bytes are: {}",bytes);
    println!("Indexed words: {}",index.words.len());
    println!("Indexed numbers: {}",index.numbers.len());
}