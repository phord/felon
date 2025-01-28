// A rules engine for applying styles to log lines.
//
// - ANSI normalization / filtering
// - Regex color markup with custom styles
// - Text modification / snipping


use std::collections::HashMap;

use indexed_file::IndexedLog;
use regex::Regex;

use super::{styled_line::{PattColor, StyledLine}, GrokLineIterator, LineViewMode};

pub struct Stylist {
    pub mode: LineViewMode,
    // Map of regex -> color pattern
    // TODO: Use PattColor::Plain for uncolored text;  PattColor::NoCrumb for colored output.
    pub patt: PattColor,
    pub matchers: Vec<Style>,
    pub named_styles: HashMap<String, PattColor>,
}

impl Stylist {
    pub fn new(mode: LineViewMode, patt: PattColor) -> Self {
        let mut stylist = Self {
            mode,
            patt,
            matchers: Vec::new(),
            named_styles: HashMap::new(),
        };

        stylist.hack_sample_matchers();

        stylist
    }

    pub fn hack_sample_matchers(&mut self) {
        // stylist.add_match(Regex::new(r"[0-9A-F]{12}(?P<red>[0-9A-F]*)").unwrap(), PattColor::Semantic);
        // stylist.add_style("red", PattColor::Number(Color::Red));

        let core_log = r"(?x)
            (?P<timestamp>   ^(...\ [\ 1-3]\d\ [0-2]\d:[0-5]\d:\d{2}\.\d{3})\ )    # date & time
            (?P<pid>          ([A-F0-9]{12})\ )                                    # PID
            (?P<crumb>        [A-Z]\ +)                                            # crumb
            (?P<module>       ([A-Za-z0-9_.]+)\ )                                  # module
            ";

        let ids = r"(?x)(?P<submodule>[a-z_.]+::[a-z_.]+ | [a-z_.]+[_.][a-z_.]+ | (?:\[([a-z0-9_.]+)\]))";

        let numbers = r"(?x)
                (?P<number>
                      \b0x[[:xdigit:]]+\b                       # 0xabcdef...
                    | \b[0-9A-F]{16}\b                          # ABCDEF1234567890  <-- 16 nibbles
                    | \b[[:xdigit:]]{8}(?:-[[:xdigit:]]{4}){3}-[[:xdigit:]]{12}\b # UUID style
                    | \b(?:[[:digit:]]+\.)*[[:digit:]]+         # integers, decimals, not part of any word, or with a suffix
                )
            ";

        self.add_match(Regex::new(core_log).unwrap(), PattColor::None);
        self.add_style("timestamp", PattColor::Timestamp);
        self.add_style("pid", PattColor::Semantic);
        // self.add_style("crumb", PattColor::Inverse);
        self.add_style("module", PattColor::Semantic);
        self.add_style("submodule", PattColor::Semantic);
        self.add_style("number", PattColor::Semantic);

        self.add_match(Regex::new(ids).unwrap(), PattColor::None);
        self.add_match(Regex::new(numbers).unwrap(), PattColor::None);

        self.add_match(Regex::new(r"(?P<segio>segio)").unwrap(), PattColor::Inverse);

        // FIXME: Prevent matches overlapping?  Or restrict highlights to a region, e.g. the "body" instead of the timestamp
    }

    pub fn add_match(&mut self, regex: Regex, pattern: PattColor) {
        self.matchers.push(Style{matcher: regex, pattern});
    }

    pub fn add_style(&mut self, name: &str, pattern: PattColor) {
        self.named_styles.insert(name.to_string(), pattern);
    }

    pub fn iter_range<'a, R, T>(&'a self, log: &'a mut T, range: &'a R) -> GrokLineIterator<'a, T>
    where R: std::ops::RangeBounds<usize>, T: IndexedLog
    {
        GrokLineIterator::range(log, self, range)
    }


    /// Apply each regex in self.styles and perform related actions / styling
    ///
    /// Given a pattern like "(?P<color>red|blue|green) fish", and a string like "One fish, two fish, red fish, blue fish",
    /// we will have four matches:
    /// 1. "red fish"
    /// 2. "color": "red"
    /// 3. "blue fish"
    /// 4. "color": "blue"
    ///
    /// Each of these may induce a specific style.  So we iterate over them, in the order show above. That is, we visit
    /// each match and apply its styles, then we visit each named capture and apply its styles, unless the named capture
    /// overlaps with a previous named capture.  (First named capture to match wins.)
    /// Note: A direct style associated with a capture group will always be applied, regardless of whether it overlaps.
    ///       To avoid this, use a group-style associated with the capture name which is separate from the capture itself.
    ///
    /// Expressions are defined with the `match` command, and groups are defined with the style command.
    ///
    ///    match "(?P<color>red|blue|green) fish" Green,Italic
    ///    style "color" Semantic,Bold
    pub fn apply(&self, line: &str) -> StyledLine {
        let mut styled = StyledLine::sanitize_basic(line, self.patt);

        // TODO: replace all NoCrumb styles with a Crumb style if one is later matched
        let mut named_ranges = Vec::new();

        for style in &self.matchers {
            for capture in style.matcher.captures_iter(line) {
                let matched = capture.get(0).unwrap();
                styled.apply(matched.as_str(), matched.range(), style.pattern);
                for group_name in style.matcher.capture_names().flatten() {
                    if let Some(group) = capture.name(group_name) {
                        if let Some(patt) = self.named_styles.get(group_name) {
                            let range = group.range();
                            if !itertools::any(&named_ranges, |r: &std::ops::Range<usize>| r.contains(&range.start) || range.contains(&r.start)) {
                                styled.apply(group.as_str(), group.range(), *patt);
                                named_ranges.push(group.range());
                            }
                        }
                    }
                }
            }
        }

        styled
    }
}

/// Transformation rules to apply to a named match
/// - Matcher (regex, string, timestamp, position?)
/// - Action (color, replace, delete, insert, etc.)
/// - Categorize (pid, module, timestamp, etc.)
pub enum StyleAction {
    Basic,              // Match categories and follow user config
    Sanitize,           // Sanitize unprintable control characters a-la less
    Replace(String),    // Replace string with another string
}

pub struct Style {
    pub(crate) matcher: Regex,
    pub(crate) pattern: PattColor,
}
