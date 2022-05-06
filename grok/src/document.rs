/// A wrapper for a LogFile that applies color, filtering, caching, etc.

use crossterm::style::Color;
use crate::config::Config;
use fnv::FnvHasher;
use std::hash::Hasher;
use lazy_static::lazy_static;
use regex::Regex;
use crate::styled_text::{PattColor, StyledLine};
use indexed_file::indexer::LogFile;
use std::collections::BTreeSet;

pub enum FilterType {
    FilterOut,
    FilterIn,
}

pub enum SearchType {
    SearchWord(String),
    SearchPhrase(String),
    SearchRegex(Regex),
}

struct DocFilter {
    filter_type: FilterType,
    search_type: SearchType,
}

impl DocFilter {
    fn first(&self, log: &LogFile) -> BTreeSet<usize> {
        match self.search_type {
            SearchType::SearchWord(ref word) => {
                log.search_word(word).as_ref().clone()
            }
            SearchType::SearchPhrase(ref _phrase) => {
                // TODO: parse phrase into words, build set of matches, and search for phrase
                unreachable!("TODO: implement");
            }
            SearchType::SearchRegex(ref _regex) => {
                // TODO: parse phrase into words, build set of matches, and search for phrase
                unreachable!("TODO: implement");
            }
        }
    }

    fn apply(&self, log: &LogFile, active: &BTreeSet<usize>) -> BTreeSet<usize> {
        let matches = self.first(log);
        match self.filter_type {
            FilterType::FilterOut => {
                active.difference(&matches).copied().collect()
            }
            FilterType::FilterIn => {
                active.intersection(&matches).copied().collect()
            }
        }
    }
}

impl DocFilter {
    pub fn new(filter_type: FilterType, search_type: SearchType) -> Self {
        Self {
            filter_type,
            search_type,
        }
    }
}

pub struct Document {
    // File contents
    // FIXME: Replace this with a filtered-view so we can apply filters
    // FIXME: StyledLine caching -- premature optimization?
    file: LogFile,

    // Filters are applied in order from first to last
    filters: Vec<DocFilter>,

    /// Filtered line numbers
    filtered_lines: Option<Vec<usize>>,
}

impl Document {
    pub fn new(config: Config) -> Self {
        let filename = config.filename.get(0).expect("No filename specified").clone();
        let file = LogFile::new(Some(filename)).expect("Failed to open file");
        println!("{:?}", file);

        Self {
            file,
            filters: vec![],
            filtered_lines: None, // Some(vec![5, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
        }
    }

    pub fn all_line_count(&self) -> usize {
        self.file.lines()
    }

    pub fn filtered_line_count(&self) -> usize {
        match self.filtered_lines {
            Some(ref lines) => lines.len(),
            None => self.all_line_count(),
        }
    }

    fn apply_filters(&mut self) {
        //TODO: apply lazily or partially and in a thread
        self.filtered_lines =
            if self.filters.is_empty() {
                             None
            } else {
                Some(self.filters
                            .iter()
                                .skip(1)
                                .fold(self.filters[0].first(&self.file).clone(),
                                    |acc, nxt| {
                                        nxt.apply(&self.file, &acc )
                                    }).into_iter().collect())
        };
    }

    pub fn add_filter(&mut self, filter_type: FilterType, search_type: SearchType) {
        self.filters.push(DocFilter::new(filter_type, search_type));
        self.apply_filters();
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
            // Apr  4 22:21:16.056 E8ABF4F03A6F I      vol.flush.cb ...
            static ref TIMESTAMP: Regex = Regex::new(r"(?x)
                ^(...\ [\ 1-3]\d\ [0-2]\d:[0-5]\d:\d{2}\.\d{3})\    # date & time
                 ([A-F0-9]{12})\                                    # PID
                 ([A-Z])\                                           # crumb").unwrap();

            static ref MODULE: Regex = Regex::new(r"(?x)
                 ^\ *([A-Za-z0-9_.]+)\                              # module
                 (?:\[([a-z0-9_.]+)\]){0,1}                         # submodule").unwrap();

            // Match at 0x{hex} number, any 16-digit all-uppercase hex number at word delimiters, or any decimal number which is not part of a word suffix.
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
            match self.filtered_lines {
                Some(ref lines) =>
                    if lrow < lines.len() { self.file.readline_at(lines[lrow])} else {None},
                None =>
                    self.file.readline(lrow),
            };
        line.unwrap_or("~")
    }
}