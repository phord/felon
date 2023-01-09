/// A wrapper for a LogFileLines that applies color, filtering, caching, etc.

use crossterm::style::Color;
use crate::config::Config;
use fnv::FnvHasher;
use std::hash::Hasher;
use lazy_static::lazy_static;
use regex::Regex;
use crate::styled_text::{PattColor, StyledLine};
use indexed_file::line_indexer::LogFileLines;
// use std::collections::BTreeSet;
// use std::ops::Bound::{Excluded, Unbounded};
use itertools::Itertools;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FilterType {
    FilterOut,
    FilterIn,
    Search,
}

// use std::cell::RefCell;

#[derive(Debug)]
pub enum SearchType {
    SearchRegex(Regex),
}

struct DocFilter {
    search_type: SearchType,
    matches : Vec<(usize, usize)>,
}

impl DocFilter {
    pub fn new(search_type: SearchType) -> Self {
        Self {
            search_type,
            matches : Vec::new(),
        }
    }

    // Find a line position given some offset into file
    fn after(&self, offset: usize) -> usize {
        let pos = self.matches.binary_search_by_key(&offset, |&(start, _)| start);
        match pos { Ok(t) => t, Err(e) => e,}
    }

    // Resolve a filter against a LogFileLines and store the matches
    fn bind(&mut self, log: &LogFileLines) {
        let matches =
            match self.search_type {
                SearchType::SearchRegex(ref regex) => {
                    // Search all lines for regex
                    // FIXME: search only filtered-in lines when possible
                    let mut matches = Vec::new();
                    for (line, start, end) in log.iter_lines() {
                        // TODO: For filter-out we will want the unmatched lines instead
                        if regex.is_match(&line) {
                            matches.push((start, end));
                        }
                    }
                    matches
                }
            };
        self.matches = matches;
    }

    // fn apply(&self, log: &LogFileLines, active: &BTreeSet<usize>) -> BTreeSet<usize> {
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
    filtered_lines: Vec<(usize, usize)>,

    file: LogFileLines,
}

impl Filters {
    fn new(file: LogFileLines) -> Self {

        let mut s = Self {
            filter_in: vec![],
            filter_out: vec![],
            highlight: vec![],
            filtered_lines: vec![], // Some(vec![5, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
            file
        };

        let v : Vec<_> = s.file.iter_offsets()
            .map(|(&start, &end)| (start, end))
            .collect();

        s.filtered_lines = v;

        s
    }

    fn add_filter(&mut self, filter_type: FilterType, search_type: SearchType) {
        println!("Adding filter {:?} {:?}", filter_type, search_type);
        let mut f = DocFilter::new(search_type);
        f.bind(&self.file);
        println!("Done");
        match filter_type {
            FilterType::FilterIn =>   self.filter_in.push(f),
            FilterType::FilterOut =>  self.filter_out.push(f),
            FilterType::Search =>     self.highlight.push(f),
        };
        // self.apply_filters();
    }

    fn apply_filters(&mut self) {
        // XXX: Keep filters in vectors, but keep Searches in a FnvHashMap.
        // XXX: For filter-out,
        //    1. find the maximum next line in each filter
        //    2. If the difference is small, linearly step the other filters until they match.
        //       If it's large, try a binary search.
    }

}

impl Filters {
    fn iter_includes_rev(& self, start: usize) -> Box<dyn Iterator<Item = (usize, usize)> + '_>  {
        if self.filter_in.is_empty() {
            let start = self.filtered_lines.binary_search_by_key(&start, |&(start, _)| start);
            let start = match start { Ok(t) => t, Err(e) => e,};
            Box::new(self.filtered_lines[..start]
                    .iter()
                    .map(|&(start, end)| (start, end)))
        } else {
            // Find the next line that matches any filter-in.
            Box::new(self.filter_in.iter()
                    .map(|x| x.matches[..x.after(start)].iter())
                    .kmerge()
                    .dedup()
                .map(|&(start, end)| (start, end)))
            }
    }

    fn iter_includes(& self, start: usize) -> Box<dyn Iterator<Item = (usize, usize)> + '_>  {
        if self.filter_in.is_empty() {
            let start = self.filtered_lines.binary_search_by_key(&start, |&(start, _)| start);
            let start = match start { Ok(t) => t, Err(e) => e,};
            Box::new(self.filtered_lines[start..]
                    .iter()
                    .map(|&(start, end)| (start, end)))
        } else {
            // Find the next line that matches any filter-in.
            Box::new(self.filter_in.iter()
                    .map(|x| x.matches[x.after(start)..].iter())
                    .kmerge()
                    .dedup()
                .map(|&(start, end)| (start, end)))
            }
    }
}
pub struct Document {
    // File contents
    // FIXME: Replace this with a filtered-view so we can apply filters
    // FIXME: StyledLine caching -- premature optimization?
    filters: Filters,
}

impl IntoIterator for Document {
    type Item = String;
    type IntoIter = DocumentIterator;

    fn into_iter(self) -> Self::IntoIter {
        DocumentIterator {
            doc: self,
            index: 0,
        }
    }
}

pub struct DocumentIterator {
    doc: Document,
    index: usize,
}

impl Iterator for DocumentIterator {
    type Item = String;
    fn next(&mut self) -> Option<String> {
        if self.index < self.doc.filters.file.count_lines() {
            let line = self.doc.filters.file.readline(self.index);
            self.index += 1;
            if line.is_some() {
                // FIXME: There's a better way to map an optional, right?
                Some(line.unwrap().to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Document {

    pub fn get_lines_from_rev(&self, start: usize, len: usize) -> Vec<(usize, &str)> {
        let iter = self.iter_filtered_rev(start);
        iter.take(len).collect()
    }

    pub fn get_lines_from(&self, start: usize, len: usize) -> Vec<(usize, &str)> {
        let iter = self.iter_filtered(start);
        iter.take(len).collect()
    }

    pub fn iter_start(&self, start: usize) -> impl Iterator<Item = (usize, &str)>  {
        self.iter_filtered(start)
    //     DocumentIterator::<'a> {
    //         doc: self,
    //         index: start,
    //         inner: Some(Box::new(self.filters.iter_includes()
    //             .map(|(start, end)| self.filters.file.readline_fixed(start, end).unwrap())))
    //     }
    }


    pub fn iter_filtered_rev(&self, pos: usize) -> impl Iterator<Item = (usize, &str)> {
        let i = self.filters.iter_includes(pos);
        i.map(|(start, end)| (start, self.filters.file.readline_fixed(start, end).unwrap_or("~")))
    }

    pub fn iter_filtered(&self, pos: usize) -> impl Iterator<Item = (usize, &str)> {
        let i = self.filters.iter_includes(pos);
        i.map(|(start, end)| (start, self.filters.file.readline_fixed(start, end).unwrap_or("~")))
    }
}

impl Document {
    pub fn new(config: Config) -> Self {
        let filename = config.filename.get(0).expect("No filename specified").clone();
        let file = LogFileLines::new(Some(filename)).expect("Failed to open file");
        println!("{:?}", file);


        let mut s = Self {
            filters: Filters::new(file),
        };

        // FIXME: This works for adding filters now. What about in the future?
        // filters.add_filter(FilterType::FilterOut, SearchType::SearchWord("flutter".to_string()));
        // doc.add_filter(FilterType::FilterIn, SearchType::SearchRegex(Regex::new(r"sectors").unwrap()));
        // doc.add_filter(FilterType::FilterIn, SearchType::SearchRegex(Regex::new(r"foo").unwrap()));
        // doc.add_filter(FilterType::FilterIn, SearchType::SearchRegex(Regex::new(r"segmap.segmap measurement timing").unwrap()));
        // s.add_filter(FilterType::FilterIn, SearchType::SearchRegex(Regex::new(r"flutter").unwrap()));
        s.add_filter(FilterType::FilterIn, SearchType::SearchRegex(Regex::new(r"e").unwrap()));

        s
    }

    pub fn all_line_count(&self) -> usize {
        self.filters.file.count_lines()
    }

    pub fn filtered_line_count(&self) -> usize {
        self.filters.filtered_lines.len()
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

    // // Deprecated
    // pub fn get(&mut self, lrow: usize) -> &str {
    //     let line =
    //         match self.filters.filtered_lines {
    //             Some(ref lines) =>
    //                 if lrow < lines.len() { self.filters.file.readline_at(lines[lrow])} else {None},
    //             None =>
    //                 self.filters.file.readline(lrow),
    //         };
    //     line.unwrap_or("~")
    // }
}