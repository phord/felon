/// Line filters and filter bundling logic
///
/// A filter-out removes a line when it matches
/// A filter-in includes a line when it matches
/// A filter-search doesn't include or exclude; it only highlights
/// A filter bundle applies filters in logical coordination:
///   No Filter-in:     Filter in everything
///   No Filter-out:    Filter out nothing
///   Both types present: filtered-in && !filtered-out
/// A FilterBatch represents lines matched within some offset range
///   adjacent batches can be merged
///   non-adjacent batches can be stored until they become adjacent
//
// Random ideas about context modifiers:
//   Support context lines: -A -B -C
//   Support context per filter: -A3 foo -A0 bar
//   Support filtered context: -A3:foo-related foo -A0 bar
//   Support field-related context: -A3:pid foo      <-- must predefine pid
//   Support timestamp context: -A{time:0.030} foo   <-- include 30ms after matches
//   Support lookback context: -C{seg} foo.*seg=(?<seg>[0-9]+)    <-- Include all lines matching matched "seg" string
//      more lookback context: -A{seg:3} foo.*seg=(?<seg>[0-9]+)  <-- Include following 3 lines matching same segment
//   Support related context: -A{seg:3} foo          <-- Include following 3 lines matching same segment
//         ^^^ This requires a config that identifies segid field for every different mention
//      more related context: -C{frame} foo          <-- Include all lines inside same frame
//         ^^^ This requires a config that identifies frame-grouped lines;
//              maybe we need a generic line-grouping config to match, i.e., adjacent lines w/o timestamp, mail headers, multi-line json-formatted data, etc.
//

use regex::Regex;

#[derive(Debug, Clone)]
pub enum SearchType {
    SearchRegex(Regex),
}

impl SearchType {
    fn new(s: &str) -> Result<SearchType, regex::Error> {
        let re = Regex::new(s)?;
        Result::Ok(SearchType::SearchRegex(re))
    }

    fn apply(&self, line: &str) -> bool {
        match self {
            SearchType::SearchRegex(ref regex) => {
                regex.is_match(line)
            }
        }
    }
}

#[test]
fn test_search_type() {
    let search = SearchType::new(r"foo").unwrap();
    assert!(search.apply("foo"));
    assert!(search.apply("football"));
    assert!(search.apply("Sunday Night football"));
    assert!(!search.apply("Sunday Night Football"));
    assert!(!search.apply("oof"));
    assert!(!search.apply("bar"));

    let search = SearchType::new(r"fo+").unwrap();
    assert!(search.apply("fo"));
    assert!(search.apply("football"));
    assert!(search.apply("Sunday Night football"));
    assert!(!search.apply("Sunday Night Football"));
    assert!(!search.apply("oof"));
    assert!(!search.apply("bar"));
}

struct Filters {
    // if any filter_in exist, all matching lines are included; all non-matching lines are excluded
    filter_in: Vec<SearchType>,

    // if any filter_out exist, all matching lines are excluded
    filter_out: Vec<SearchType>,

    // // Highlight-matching lines
    // highlight: Vec<DocFilter>,

    // /// Filtered line numbers
    // filtered_lines: Vec<(usize, usize)>,
}

impl Filters {
    fn new() -> Self {
        Self {
            filter_in: vec![],
            filter_out: vec![],
        }
    }

    fn add_in(&mut self, re: &str) -> Result<bool, regex::Error> {
        let search = SearchType::new(re)?;
        self.filter_in.push(search.clone());
        Result::Ok(true)
    }

    fn add_out(&mut self, re: &str) -> Result<bool, regex::Error> {
        let search = SearchType::new(re)?;
        self.filter_out.push(search.clone());
        Result::Ok(true)
    }

    fn apply(&self, line: &str) -> bool {
        let filter_in =
            if self.filter_in.is_empty() { true }
            else { self.filter_in.iter().any(|x| x.apply(line)) };

        let filter_out =
            if self.filter_out.is_empty() { false }
            else { self.filter_out.iter().any(|x| x.apply(line)) };

        filter_in && !filter_out
    }
}

#[test]
fn test_filter_in() {
    let apply = |filter: &Filters| -> Vec<bool> {
        let data = "foo\nbar\nbaz\nfrotz\nfrobnatz\n";
        data.lines().map(|line| filter.apply(line)).collect()
    };

    let mut filters = Filters::new();

    // No filters:
    assert_eq!(apply(&filters), vec![ true,  true,  true,  true,  true] );

    // grep "f"
    filters.add_in("f").unwrap();
    assert_eq!(apply(&filters), vec![ true, false, false,  true,  true] );

    // grep "z"
    filters.add_in("z").unwrap();
    assert_eq!(apply(&filters), vec![ true, false,  true,  true,  true] );
}


#[test]
fn test_filter_out() {
    let apply = |filter: &Filters| -> Vec<bool> {
        let data = "foo\nbar\nbaz\nfrotz\nfrobnatz\n";
        data.lines().map(|line| filter.apply(line)).collect()
    };

    let mut filters = Filters::new();

    // No filters:
    assert_eq!(apply(&filters), vec![ true,  true,  true,  true,  true] );

    // grep "f"
    filters.add_out("f").unwrap();
    assert_eq!(apply(&filters), vec![ false,  true,  true, false, false] );

    // grep "z"
    filters.add_out("z").unwrap();
    assert_eq!(apply(&filters), vec![ false,  true, false, false, false] );
}

#[test]
fn test_filter_in_out() {
    let apply = |filter: &Filters| -> Vec<bool> {
        let data = "foo\nbar\nbaz\nfrotz\nfrobnatz\n";
        data.lines().map(|line| filter.apply(line)).collect()
    };

    let mut filters = Filters::new();
    filters.add_in("f").unwrap();    // grep "f"
    filters.add_out("z").unwrap();   // grep -v "z"
    assert_eq!(apply(&filters), vec![  true, false, false, false, false] );
}

/*
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
                    .cloned())
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
*/