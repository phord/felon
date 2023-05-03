// A factory for timestamps for log lines in a file
use chrono::NaiveDateTime;

pub struct TimeStamper {
    pub patterns: Vec<String>,
    matches: Vec<usize>,
    unmatched: usize,
}

impl TimeStamper {
    // TODO: Add configs
    pub fn default() -> Self {
        Self {
            patterns: Vec::default(),
            matches: Vec::default(),
            unmatched: 0,
        }
    }

    pub fn push(&mut self, matcher: String) {
        self.patterns.push(matcher);
        self.matches.push(0);
        self.unmatched = 0;
    }

    pub fn time(&mut self, line: &String) -> Option<NaiveDateTime> {
        // let parse_from_str = NaiveDateTime::parse_from_str;
        // assert_eq!(parse_from_str("2015-09-05 23:56:04", "%Y-%m-%d %H:%M:%S"),
        //            Ok(NaiveDate::from_ymd_opt(2015, 9, 5).unwrap().and_hms_opt(23, 56, 4).unwrap()));

        // Pack this into a time factory we can hand out per file.
        // Then, try each configured timestamp matcher in turn until we find one.
        // Count how many successful matches there are for each.
        // When one of them reaches 1000 (or some threshold) assume that's the format and stop checking the others.
        // If none matches after (threshold) tests, stop testing all of them and assume None for every line.

        // Apr  4 22:21:15.813
        // lnav matcher: "^(?<timestamp>[A-Z][a-z]{2} {1,2}\\d{1,2} \\d{2}:\\d{2}:\\d{2}\\.\\d{3}) (?<pid>[0-9A-F]{12}) (?<crumb>[A-Z])      "
        for (i, m) in self.patterns.iter().enumerate() {
            match NaiveDateTime::parse_from_str(m, line.as_str()) {
                Ok(t) => {
                    self.matches[i] += 1;
                    // TODO: Compare to threshold for "winning matcher" and destroy all others when reached
                    return Some(t)
                },
                _ => {},
            }
        }

        // TODO: Compare to threshold for "max losers" and turn off checking if reached
        self.unmatched += 1;

        None
    }
}