// A collection of log lines from multiple log files.
// Lines can be blended by merging (sorted) or by concatenation.

struct Doc {
    files: Vec<LineIndexer>
}

impl Doc {
    fn new(files: Vec<PathName>) -> Result(Self) {
        todo!();
    }
}