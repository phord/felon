/// A wrapper for a LogFile that applies color, filtering, caching, etc.

use crossterm::style::Color;
use crate::config::Config;
use fnv::FnvHasher;
use std::hash::Hasher;
use lazy_static::lazy_static;
use regex::Regex;
use crate::styled_text::{PattColor, StyledLine};
use indexed_file::indexer::LogFile;
// use std::collections::BTreeSet;
// use std::ops::Bound::{Excluded, Unbounded};
// use itertools::Itertools;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FilterType {
    FilterOut,
    FilterIn,
    Search,
}

// use std::cell::RefCell;

pub enum SearchType {
    SearchWord(String),
    SearchPhrase(String),
    SearchRegex(Regex),
}

struct DocFilter {
    filter_type: FilterType,
    search_type: SearchType,
    matches : Vec<usize>,
}

impl DocFilter {
    pub fn new(filter_type: FilterType, search_type: SearchType) -> Self {
        Self {
            filter_type,
            search_type,
            matches : Vec::new(),
        }
    }

    // Find partition point of a line position
    fn after(&self, offset: usize) -> usize {
        self.matches.binary_search(&offset).unwrap()
    }

    // Resolve a filter against a LogFile and store the matches
    fn bind(&mut self, log: &LogFile) {
        let matches =
            match self.search_type {
                SearchType::SearchWord(ref word) => {
                    unreachable!("TODO: FIXME");
                    // Vec::<usize>::from(log.search_word(word).clone().into_iter())
                }
                SearchType::SearchPhrase(ref _phrase) => {
                    // TODO: parse phrase into words, build set of matches, and search for phrase
                    unreachable!("TODO: implement");
                    // BTreeSet::<usize>::new()
                }
                SearchType::SearchRegex(ref regex) => {
                    // Search all lines for regex
                    // FIXME: search only filtered-in lines when possible
                    let mut matches = Vec::<usize>::new();
                    for l in 0..log.count_lines() {
                        // FIXME: read by offset and return offset along with line so we don't have to look it up again
                        let line = log.readline(l);
                        if let Some(line) = line {
                            if regex.is_match(&line) {
                                matches.push(log.line_offset(l).unwrap());
                            }
                        }
                    }
                    matches
                }
            };
        self.matches = matches;
    }

    // fn apply(&self, log: &LogFile, active: &BTreeSet<usize>) -> BTreeSet<usize> {
    //     let matches = self.first(log);
    //     match self.filter_type {
    //         FilterType::FilterOut => {
    //             active.difference(&matches).copied().collect()
    //         }
    //         FilterType::FilterIn => {
    //             active.union(&matches).copied().collect()
    //         }
    //         FilterType::Search => {
    //             active.intersection(&matches).copied().collect()
    //         }
    //     }
    // }
}
struct Filters {
    // if any filter_in exist, all matching lines are included; all non-matching lines are excluded
    filter_in: Vec<DocFilter>,

    // if any filter_out exist, all matching lines are excluded
    filter_out: Vec<DocFilter>,

    // Highlight-matching lines, even if they're filtered out
    highlight: Vec<DocFilter>,

    /// Filtered line numbers
    filtered_lines: Option<Vec<usize>>,

}

impl Filters {
    pub fn new() -> Self {

        Self {
            filter_in: vec![],
            filter_out: vec![],
            highlight: vec![],
            filtered_lines: None, // Some(vec![5, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
        }
    }

    pub fn add_filter(&mut self, filter_type: FilterType, search_type: SearchType) {
        let f = DocFilter::new(filter_type, search_type);
        match filter_type {
            FilterType::FilterIn =>   self.filter_in.push(f),
            FilterType::FilterOut =>  self.filter_out.push(f),
            FilterType::Search =>     self.highlight.push(f),
        };
        self.apply_filters();
    }

    fn apply_filters(&mut self) {
        // //TODO: apply lazily or partially and in a thread
        // self.filtered_lines =
        //     if self.filters.is_empty() {
        //                      None
        //     } else {
        //         // FIXME: Filter-out is not working for single filter. (also for multiple?)
        //         // FIXME: Lazy-eval filters by just keeping vectors of line offsets. Then iterate lines
        //         // by finding numbers in common between the sets.
        //         // XXX: Keep filters in vectors, but keep Searches in a FnvHashMap.
        //         // XXX: For filters,
        //         //    1. find the maximum next line in each filter
        //         //    2. If the difference is small, linearly step the other filters until they match.
        //         //       If it's large, try a binary search.
        //         let first = self.filters[0].first(&self.file).clone();
        //         Some(self.filters
        //                     .iter()
        //                         .skip(1)
        //                         .fold(first,
        //                             |acc, nxt| {
        //                                 nxt.apply(&self.file, &acc )
        //                             }).into_iter().collect())
        // };
    }

}

pub struct Document {
    // File contents
    // FIXME: Replace this with a filtered-view so we can apply filters
    // FIXME: StyledLine caching -- premature optimization?
    file: LogFile,
    filters: Filters,
}

impl<'a> IntoIterator for &'a Document {
    type Item = &'a str;
    type IntoIter = DocumentIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DocumentIterator {
            doc: self,
            index: 0,
        }
    }
}

impl<'a> Document {
    pub fn iter_start(&'a self, start: usize) -> <&'a Document as IntoIterator>::IntoIter {
        DocumentIterator::<'a> {
            doc: self,
            index: start,
        }
    }
}

pub struct DocumentIterator<'a> {
    doc: &'a Document,
    index: usize,
}

impl<'a> Iterator for DocumentIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.index < self.doc.file.count_lines() {
            let line = self.doc.file.readline(self.index);
            self.index += 1;
            line
        } else {
            None
        }
    }
}

impl Document {
    pub fn new(config: Config) -> Self {
        let filename = config.filename.get(0).expect("No filename specified").clone();
        let file = LogFile::new(Some(filename)).expect("Failed to open file");
        println!("{:?}", file);

        Self {
            file,
            filters: Filters::new(),
        }
    }

    pub fn all_line_count(&self) -> usize {
        self.file.count_lines()
    }

    pub fn filtered_line_count(&self) -> usize {
        match self.filters.filtered_lines {
            Some(ref lines) => lines.len(),
            None => self.all_line_count(),
        }
    }

    pub fn add_filter(&mut self, filter_type: FilterType, search_type: SearchType) {
        self.filters.add_filter(filter_type, search_type)
    }

    fn hash_color(&self, text: &str) -> Color {
        let mut hasher = FnvHasher::default();
        hasher.write(text.as_bytes());
        let hash = hasher.finish();

        let base = 0x80 as u8;
        let red = (hash & 0xFF) as u8 | base;
        let green = ((hash >> 8) & 0xFF) as u8 | base;
        let blue = ((hash >> 16) & 0xFF) as u8 | base;

        Color::Rgb {r: red, g: green, b: blue}
    }

    pub fn line_colors(&self, line: &str) -> StyledLine {
        lazy_static! {
            // TODO: Move these regexes to a config file
            // Apr  4 22:21:16.056 E8ABF4F03A6F I      vol.flush.cb ...
            static ref TIMESTAMP: Regex = Regex::new(r"(?x)
                ^(...\ [\ 1-3]\d\ [0-2]\d:[0-5]\d:\d{2}\.\d{3})\    # date & time
                 ([A-F0-9]{12})\                                    # PID
                 ([A-Z])\                                           # crumb").unwrap();

            static ref MODULE: Regex = Regex::new(r"(?x)
                 ^\ *([A-Za-z0-9_.]+)\                              # module
                 (?:\[([a-z0-9_.]+)\]){0,1}                         # submodule").unwrap();

            // Match any 0x{hex} number, any 16-digit all-uppercase hex number at word delimiters, or any decimal number which is not part of a word suffix.
            static ref NUMBER: Regex = Regex::new(r"[^A-Za-z.0-9_](\b0x[[:xdigit:]]+\b|\b[0-9A-F]{16}\b|(?:[[:digit:]]+\.)*[[:digit:]]+)").unwrap();
        }
        let prefix = TIMESTAMP.captures(line);

        let mut styled = StyledLine::new(line, PattColor::NoCrumb);

        // Match and color PID and TIME
        let mut pos = 0;
        if let Some(p) = prefix {
            let crumb = p.get(3).unwrap().as_str();
            let default_style = match crumb.as_ref() {
                "E" => PattColor::Error,
                "A" => PattColor::Fail,
                _ => PattColor::Info,
            };

            styled.push(0, line.len(), default_style);

            let len = p.get(1).unwrap().end() + 1;
            styled.push(0, len, PattColor::Timestamp);

            // TODO: Calculate timestamp value?

            let pid = p.get(2).unwrap();
            let start = pid.start();
            let end = pid.end();
            let pid = pid.as_str();
            let pid_color = self.hash_color(pid);
            styled.push( start, end, PattColor::Pid(pid_color));

            // Match modules at start of line
            pos = end + 3;  // Skip over crumb; it will autocolor later
            let module = MODULE.captures(&line[pos..]);
            if let Some(m) = module {
                let first = m.get(1).unwrap();
                let color = self.hash_color(first.as_str());
                styled.push(pos + first.start(), pos + first.end(),PattColor::Module(color) );

                if let Some(second) = m.get(2) {
                    let color = self.hash_color(second.as_str());
                    styled.push(pos + second.start(), pos + second.end(), PattColor::Module(color));
                }
            }
        }

        for m in NUMBER.captures_iter(&line[pos..]) {
            let m = m.get(1).unwrap();
            let start = m.start();
            let end = m.end();
            let color = self.hash_color(m.as_str());
            styled.push( pos + start, pos + end , PattColor::Number(color) );
        }

        styled
    }

    pub fn get(&mut self, lrow: usize) -> &str {
        let line =
            match self.filters.filtered_lines {
                Some(ref lines) =>
                    if lrow < lines.len() { self.file.readline_at(lines[lrow])} else {None},
                None =>
                    self.file.readline(lrow),
            };
        line.unwrap_or("~")
    }
}