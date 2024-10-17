use regex::Regex;

use crate::{indexer::{eventual_index::{Location, VirtualLocation}, line_indexer::IndexedLog}, Log, LogLine};


/**
 * Filter should be stackable; that is, it should be a log-line-producer, and it should take a log-line-producer.
 */

trait FilterLink {
    // Iterate lines that match our filter condition (with memoization)
    // def fn iter_matched() -> DoubleEndedIterator<LogLine>;

    // Iterate all lines without filtering. Used by upstream links to count filter-matches in
    // filtered lines
    // def fn iter_all() -> DoubleEndedIterator<LogLine>;
}

pub struct Filter {
    ins: RegexSet,
    outs: RegexSet,
}

#[inline]
fn is_match_type(line: &LogLine, typ: &FilterType) -> bool {
    match typ {
        FilterType::Regex(re) => re.is_match(line),
    }
}

impl Filter {
    pub fn new() -> Self {
        Filter { ins: RegexSet::new(), outs: RegexSet::new() }
    }

    pub(crate) fn push(&mut self, mode: FilterMode, typ: FilterType) {
        match mode {
            FilterMode::In => self.ins.push(typ),
            FilterMode::Out => self.outs.push(typ),
        }
    }

    pub fn is_match(&self, line: &LogLine) -> bool {
        self.ins.is_match(line, true) && !self.outs.is_match(line, false)
    }
}

// RegexFilter holds a single regex filter
pub struct RegexFilter {
    re: Regex,
}

pub(crate) enum FilterMode {
    In,
    Out,
}

pub(crate) enum FilterType {
    Regex(RegexFilter),
}

impl RegexFilter {
    pub fn new(expr: String) -> Result<Self, regex::Error> {
        Ok(RegexFilter {
            re: Regex::new(&expr)?,
        })
    }

    pub fn is_match(&self, line: &LogLine) -> bool {
        self.re.is_match(&line.line)
    }
}

pub struct RegexSet {
    filters: Vec<FilterType>,
}

impl RegexSet {
    pub fn new() -> Self {
        RegexSet { filters: vec![] }
    }

    fn push(&mut self, f: FilterType) {
        self.filters.push(f)
    }

    fn is_match(&self, line: &LogLine, default: bool) -> bool {
        if self.filters.is_empty() {
            default
        } else {
            self.filters.iter().any(|f| is_match_type(line, f))
        }
    }
}


// XXX: Can this be a wrapper like DataIterator is?  We could just store Locations for matched lines...
pub(crate) struct FilteredIterator<'a> {
    log: &'a mut Log,
    pos: Location,
    rev_pos: Location,
}

impl<'a> FilteredIterator<'a> {
    pub(crate) fn new(log: &'a mut Log) -> Self {
        Self {
            log,
            pos: Location::Virtual(VirtualLocation::Start),
            rev_pos: Location::Virtual(VirtualLocation::End),
        }
    }
}

impl<'a> FilteredIterator<'a> {
    pub(crate) fn new_from(log: &'a mut Log, offset: usize) -> Self {
        let rev_pos = Location::Virtual(VirtualLocation::Before(offset));
        let pos = Location::Virtual(VirtualLocation::AtOrAfter(offset));
        Self {
            log,
            pos,
            rev_pos,
        }
    }

    fn iterate(&mut self, pos: Location) -> (Location, Option<usize>) {
        let pos = self.log.file.resolve_location(pos);

        let ret = pos.offset();
        if self.rev_pos == self.pos {
            // End of iterator when fwd and rev meet
            self.rev_pos = Location::Invalid;
            self.pos = Location::Invalid;
            (Location::Invalid, ret)
        } else {
            (pos, ret)
        }
    }

    // Read and timestamp a string at a given start from our log source
    #[inline]
    fn read_line(&mut self, offset: usize) -> std::io::Result<LogLine> {
        let line = self.log.file.read_line_at(offset)?;
        Ok(LogLine::new( line, offset ))
    }
}

impl<'a> Iterator for FilteredIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.pos);
        self.pos = self.log.file.next_line_index(pos);
        if ret.is_some() && ret.unwrap() >= self.log.file.len() {
            None
        } else {
            ret
        }
    }
}

impl<'a> DoubleEndedIterator for FilteredIterator<'a> {
    // Iterate over lines in reverse
    fn next_back(&mut self) -> Option<Self::Item> {
        let (pos, ret) = self.iterate(self.rev_pos);
        self.rev_pos = self.log.file.prev_line_index(pos);
        ret
    }
}
