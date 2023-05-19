use regex::RegexSet;


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
    src: Vec<String>,
    re: Option<RegexSet>,
    log: Box<dyn FilterLink>,
}

impl Filter {
    pub fn new<LOG: FilterLink>(log: LOG) -> Self {
        Filter { src: vec![], re: None, log: Box::new(log) }
    }

    pub fn is_match(&self) -> bool {
        false
    }
}