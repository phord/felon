/// Support different line clipping modes:
///  - Chop: break lines at exactly the last byte that fits on the line; show remainder on next line
///  - Clip: clip the leading and trailing portions of the line; do not show remainder
///  - WholeLine: show the whole line; assumes the display will handle wrapping somehow
///  - Wrap: (TODO) wrap text at work breaks
///
///  TODO: add continuation indent option for chopped/wrapped lines
#[derive(Clone, Copy, Debug)]
pub enum LineViewMode{
    Wrap{width: usize},
    Clip{width: usize, left: usize},
    WholeLine,
}

impl LineViewMode {
    // Test if this line may be displayed in multiple chunks (wrapped)
    pub fn is_chunked(&self) -> bool {
        matches!(self, LineViewMode::Wrap{width: _})
    }

    /// Return the start of the chunk we're on, given an arbitrary offset into the line
    pub fn chunk_start(&self, index: usize) -> usize {
        match self {
            LineViewMode::Wrap{width} => index - index % width,
            LineViewMode::Clip{width: _, left} => *left,
            LineViewMode::WholeLine => 0,
        }
    }

    /// Return the end of the chunk we're on, given an arbitrary offset into the line
    pub fn chunk_end(&self, start: usize, end: usize) -> usize {
        match self {
            LineViewMode::Wrap{width} => end.min(start + *width),
            LineViewMode::Clip{width, left: _} => end.min(start + *width),
            LineViewMode::WholeLine => end,
        }
    }
}
