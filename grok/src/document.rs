/// A wrapper for a LogFileLines that applies color, filtering, caching, etc.

use crate::{config::Config, styled_text::{styled_line::{PattColor, StyledLine}, stylist::Stylist, LineViewMode}};
use regex::Regex;
use indexed_file::{files::Stream, indexer::indexed_log::IndexStats, IndexedLog, Log, LogLine, LogStack};
pub struct Document {
    // FIXME: StyledLine caching -- premature optimization?
    // File contents
    log: LogStack,
    stylist: Stylist,
}

impl Document {

    pub fn get_lines_range<'a, R>(&'a mut self, range: &'a R) -> impl DoubleEndedIterator<Item = LogLine> + 'a
    where R: std::ops::RangeBounds<usize> {
        self.stylist.iter_range(&mut self.log, range)
    }

    pub fn get_plain_lines<'a, R>(&'a mut self, range: &'a R) -> impl DoubleEndedIterator<Item = LogLine> + 'a
    where R: std::ops::RangeBounds<usize> {
        self.log.iter_lines_range(range)
    }

    pub fn set_search(&mut self, search: &str) -> Result<(), regex::Error> {
        self.stylist.remove_match(crate::styled_text::StyleReason::Search);
        self.stylist.add_match(crate::styled_text::StyleReason::Search, Regex::new(search)?, PattColor::Inverse);
        // TODO: force viewer to refresh page
        self.log.search_regex(search)
    }

    pub fn clear_filter(&mut self) -> Result<(), regex::Error> {
        self.log.filter_regex("")
    }

    pub fn set_filter(&mut self, filter: &str) -> Result<(), regex::Error> {
        self.log.filter_regex(filter)
    }

    pub fn search_next(&mut self, line: usize, repeat: usize) -> Option<usize> {
        self.log.search_next(repeat, line)
    }

    pub fn search_back(&mut self, line: usize, repeat: usize) -> Option<usize> {
        self.log.search_next_back(repeat, line)
    }

    pub fn run(&mut self, timeout: u64) -> Option<usize> {
        self.log.run_pending(timeout)
    }

    pub fn describe_pending(&self) -> String {
        self.log.describe_pending()
    }

    pub fn has_pending(&self) -> bool {
        self.log.has_pending()
    }

    pub fn poll(&mut self, timeout: Option<std::time::Instant>) -> usize {
        self.log.poll(timeout)
    }
}

impl Document {
    pub fn new(config: Config) -> Self {
        let filename = config.filename.first();
        let log = Log::open(filename).expect("Failed to open file");

        Self {
            log: LogStack::new(log),
            stylist: Stylist::default(),
        }
    }

    pub fn set_line_mode(&mut self, mode: LineViewMode) {
        self.stylist.mode = mode;
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.log.len()
    }

    pub fn info(&self) -> impl Iterator<Item = &IndexStats> + '_
    where Self: Sized
    {
        self.log.info()
    }

    // FIXME: Move to Stylist?
    // This is now used only for ~ lines at end.
    pub fn line_colors(&self, line: &str) -> StyledLine {
        StyledLine::new(line, PattColor::NoCrumb)
    }
}
