// A partial index that maps all the linefeeds in a chunk of data
// Each index knows the offset for its chunk into the original data.  So looking up a
// a line number will return the offset into the original data, not just the chunk.

use std::io::BufRead;


pub struct Index {
    // Offset of buffer we indexed
    pub start: usize,
    // End is inclusive or exclusive depending on context:
    //   Offset of byte after buffer we indexed (exclusive)
    //   Last line offset included in our range (inclusive)
    pub end: usize,
    // Byte offset of the end of each line
    line_offsets: Vec<usize>,
}

impl Index {
    pub fn new() -> Index {
        // FIXME: pass start/end here and set it once. Don't let parse() set it because it can change over multiple calls.
        Index {
            start: 0,
            end: 0,
            line_offsets: Vec::new(),
        }
    }

    pub fn bytes(&self) -> usize {
        self.end - self.start
    }

    pub fn lines(&self) -> usize {
        self.line_offsets.len()
    }

    // "Empty" means we found no linefeeds, even if our chunk size is non-zero
    pub fn is_empty(&self) -> bool {
        self.line_offsets.is_empty()
    }

    pub fn get(&self, line_number: usize) -> usize {
        assert!(line_number < self.len());
        self.line_offsets[line_number]
    }

    pub fn len(&self) -> usize {
        self.line_offsets.len()
    }

    // Accumulate the map of line offsets into self.line_offsets
    // Parse buffer passed in using `offset` as index of first byte
    pub fn parse(&mut self, data: &[u8], offset: usize) {
        let end = offset + data.len();
        if self.end == 0 {
            // New index; accept anything
            assert_eq!(self.start, 0);
            assert!(self.is_empty());
            self.start = offset;
            self.end = end;
        } else if self.end == offset {
            // Contiguous data added to end
            self.end = end;
        } else if self.start == end {
            // Contiguous data added to front
            self.start = offset;
            panic!("contiguous blocks parsed in reverse order is untested");
        } else {
            panic!("multiple parsed blocks must be contiguous: {}..{} and {}..{}", self.start, self.end, offset, end);
        }

        // Special case for start of file: include first line
        if offset == 0 {
            self.line_offsets.extend(std::iter::once(0));
        }

        // Store line beginnings
        let newlines = data
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == b'\n')
            .map(|(i, _)| i + offset + 1);
        self.line_offsets.extend(newlines);
    }

    // Parse lines from a BufRead
    pub fn parse_bufread<R: BufRead>(&mut self, source: &mut R, offset: usize, len: usize) -> std::io::Result<usize> {
        /* Alternative:
            let mut pos = offset;
            let newlines = source.lines()
                .map(|x| { pos += x.len() + 1; pos });
            self.line_offsets.extend(newlines);
         */
        let mut pos = offset;
        let end = offset + len;
        while pos < end {
            let bytes =
                match source.fill_buf() {
                    Ok(buf) => {
                        if buf.len() == 0 {
                            break       // EOF
                        }
                        let len = buf.len().min(end - pos);
                        self.parse(&buf[..len], pos);
                        len
                    },
                    Err(e) => {
                        return std::io::Result::Err(e)
                    },
                };
            pos += bytes;
            source.consume(bytes);
        }
        Ok(pos - offset)
    }

    pub fn iter(self: &Self) -> impl DoubleEndedIterator<Item = &usize> {
        self.line_offsets.iter()
    }

    // Find the line with a given offset using a binary_search
    // Should this be a trait?
    pub fn binary_search(self: &Self, offset: usize) -> Result<usize, usize> {
        self.line_offsets.binary_search(&offset)
    }

    pub fn find(self: &Self, offset: usize) -> Option<usize> {
        if offset < self.start || offset > self.end {
            None
        } else {
            match self.binary_search(offset) {
                Ok(line) => Some(line),
                Err(line) => Some(line),
            }
        }
    }

    // TODO: Is there a standard trait for this?
    pub fn contains_offset(&self, offset: &usize) -> std::cmp::Ordering {
        if offset > &self.end {
            std::cmp::Ordering::Less
        } else if offset < &self.start {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    }

    #[inline(always)]
    pub fn contains(&self, offset: &usize) -> bool {
        match self.contains_offset(offset) {
            std::cmp::Ordering::Equal => true,
            _ => false
        }
    }


}

// Tests for Index
#[cfg(test)]
mod tests {
    use crate::indexer::index::Index;
    static STRIDE: usize = 11;
    static DATA: &str = "0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n";
    static END: usize = DATA.len();
    static OFFSETS:[usize; 8] = [0, 11,22,33,44,55,66,77];

    // Verify index.line_offsets match expected set only in the range [start, end]
    fn check_partial(index: &Index, start:usize, end: usize) {
        let offsets: Vec<usize> =
            OFFSETS
                .iter()
                .filter(|x| **x >= start && **x <= end)
                .cloned()
                .collect();
        assert_eq!(index.iter().cloned().collect::<Vec<usize>>(), offsets);
    }

    #[test]
    fn test_index_whole_file() {
        let mut index = Index::new();
        index.parse(DATA.as_bytes(), 0);
        check_partial(&index, 0, END);
    }

    #[test]
    fn test_index_first_part() {
        let mut index = Index::new();
        index.parse(DATA[..END/2].as_bytes(), 0);
        assert!(END/2 % STRIDE > 0 );
        check_partial(&index, 0, END / 2);
    }

    #[test]
    fn test_index_empty() {
        let mut index = Index::new();
        index.parse(DATA[..STRIDE-1].as_bytes(), 0);
        assert!(!index.is_empty());

        index.parse(DATA[STRIDE-1..STRIDE].as_bytes(), STRIDE-1);
        assert!(!index.is_empty());
        check_partial(&index, 0, STRIDE);
    }

    #[test]
    fn test_index_at_zero() {
        let mut index = Index::new();
        index.parse(DATA[..STRIDE-1].as_bytes(), 0);
        assert!(!index.is_empty(), "We should index first line even if no other lines");

        let mut index = Index::new();
        index.parse(DATA[..0].as_bytes(), 0);
        assert!(!index.is_empty(), "We should index first line even when empty");

        let mut index = Index::new();
        index.parse(DATA[..STRIDE-1].as_bytes(), 1);
        assert!(index.is_empty(), "We should not index first line except at offset zero");
    }

    #[test]
    fn test_index_middle() {
        // Assumes the prev chunk matched the first line, so this matches only the 2nd and 3rd lines.
        let mut index = Index::new();
        let start = STRIDE - 1;
        let end = STRIDE + 2;
        index.parse(DATA[start..end].as_bytes(), start);
        check_partial(&index, start, end);
    }

    #[test]
    fn test_index_middle_to_end() {
        // Assumes the prev chunk matched the first line, so this matches the 2nd line until the end.
        let mut index = Index::new();
        let start = STRIDE + 1;
        index.parse(DATA[start..END].as_bytes(), start);
        check_partial(&index, start, END);
    }

    #[test]
    fn test_index_all_chunks() {
        // Try every chunk size and assemble an entire map
        for chunk in (STRIDE..END).rev() {
            let mut index = Index::new();
            // FIXME: What's the rust way to do chunk windows?
            for start in 0..=END/chunk {
                let start = start * chunk;
                let end = start + chunk;
                let end = end.min(END);
                index.parse(DATA[start..end].as_bytes(), start);
            }
            check_partial(&index, 0, END);
        }
    }

    #[test]
    fn test_iterator() {
        let mut index = Index::new();
        index.parse(DATA.as_bytes(), 0);
        assert!(index.iter().count() == OFFSETS.len());
        check_partial(&index, 0, END);
    }

    #[test]
    fn test_iterator_reverse() {
        let mut index = Index::new();
        index.parse(DATA.as_bytes(), 0);
        assert!(index.iter().rev().count() == OFFSETS.len());
    }
}
