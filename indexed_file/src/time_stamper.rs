// A factory for timestamps for log lines in a file
use chrono::{NaiveDateTime, NaiveDate, NaiveTime};
use regex::Regex;

pub struct TimeStamper {
    pub patterns: Vec<Regex>,
    matches: Vec<usize>,
    unmatched: usize,
}

impl TimeStamper {
    // TODO: Add configs
    pub fn default() -> Self {
        // TODO: Load from config

        // TODO: Use RegexSet here
        let src_patterns = vec![
            r"^(?P<month>Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) (?P<day>[ 0-9]{2}) (?P<clock>[0-2][0-9]:[0-5][0-9]:[0-6][0-9]\.\d{3})\b",
        ];

        let mut s = Self {
            patterns: Vec::default(),
            matches: Vec::default(),
            unmatched: 0,
        };

        for p in src_patterns {
            s.push(p);
        }
        s
    }

    pub fn push(&mut self, matcher: &str) {
        match Regex::new(&matcher) {
            Ok(re) => {
                self.patterns.push(re);
                self.matches.push(0);
                self.unmatched = 0;
            },
            e => eprintln!("Error parsing timestamp pattern: {:?}", e),
        }
    }

    fn parse_time(line: &str, re: &Regex) -> Option<NaiveDateTime> {

        let months = "Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec";

        if let Some(caps) = re.captures(line) {
            let month:u32 = match caps.name("month") {
                Some(x) => {
                    let m = x.as_str().trim();
                    if let Ok(imonth) = m.parse::<u32>() {
                        imonth
                    } else {
                        // Find month name string and convert to 1..12
                        months.find(m).unwrap_or(0) as u32 / 4 + 1
                    }
                },
                _ => return None,
            };

            let day:u32 = match caps.name("day") {
                Some(x) => {
                    if let Ok(day) = x.as_str().trim().parse::<u32>() {
                        day
                    } else {
                        return None
                    }
                },
                _ => return None,
            };

            let clock = match caps.name("clock") {
                Some(x) => {
                    let clock = x.as_str();
                    let time = NaiveTime::parse_from_str(clock, "%H:%M:%S%.3f");
                    time.unwrap()

                },
                _ => return None,
            };

            if let Some(date) = NaiveDate::from_ymd_opt(2000, month, day) {
                Some(NaiveDateTime::new(date, clock))
            } else {
                None
            }
        } else {
            None
        }
    }


    pub fn time(&mut self, line: &str) -> Option<NaiveDateTime> {
        // TODO: Pack this into a time factory we can hand out per file.
        // Then, try each configured timestamp matcher in turn until we find one.
        // Count how many successful matches there are for each.
        // When one of them reaches 1000 (or some threshold) assume that's the format and stop checking the others.
        // If none matches after (threshold) tests, stop testing all of them and assume None for every line.

        // Apr  4 22:21:15.813
        // Mmm DD HH:MM:SS.fff
        // lnav matcher: "^(?<timestamp>[A-Z][a-z]{2} {1,2}\\d{1,2} \\d{2}:\\d{2}:\\d{2}\\.\\d{3}) (?<pid>[0-9A-F]{12}) (?<crumb>[A-Z])      "
        for (i, m) in self.patterns.iter().enumerate() {
            if let Some(ts) = TimeStamper::parse_time(line, m) {
                self.matches[i] += 1;
                // TODO: Compare to threshold for "winning matcher" and destroy all others when reached
                return Some(ts)
            }
        }

        // TODO: Compare to threshold for "max losers" and turn off checking if reached
        self.unmatched += 1;

        None
    }
}

#[test]
fn test_timestamp_fields() {
    use chrono::{Datelike, Timelike};
    let mut stamper = TimeStamper::default();
    let line = "Apr  7 22:21:15.813 some log data here";

    let time = stamper.time(line).unwrap();
    assert_eq!(time.month(), 4);
    assert_eq!(time.day(), 7);
    assert_eq!(time.hour(), 22);
    assert_eq!(time.minute(), 21);
    assert_eq!(time.second(), 15);
    assert_eq!(time.timestamp_subsec_millis(), 813);
}

#[test]
fn test_timestamp_fail() {
    let mut stamper = TimeStamper::default();

    // All these timestamps are invalid
    let lines = vec![
        "APR  7 22:21:15.813 ",
        "April 7 22:21:15.813 ",
        "Apr  32 22:21:15.813 ",
        "Foo  2 22:21:15.813 ",
        "Foo  2 24:21:15.813 ",
        "  { no timestamp here ...",
    ];

    for line in lines {
        println!("{line}");
        assert!(stamper.time(line).is_none());
    }
}
