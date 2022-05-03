use crossterm::style::{Stylize, ContentStyle};
use std::cmp;
use crossterm::style::Color;

pub trait Stylable {
    fn stylize(&mut self, row: &str) -> ();
}

/// Defines a style for a portion of a line.  Represents the style and the position within the line.
/// The line content is included here for easier iteration, but the whole line lives elsewhere.
#[derive(Copy, Clone)]
pub struct Phrase {
    pub start: usize,
    pub end: usize,
    pub patt: PattColor,
}

/// Holds a line of text and the styles for each character.
/// The styles are stored in phrases, a sorted collection of start,end,style.
/// Phrases are not allowed to overlap. If an overlapping phrase is added, it clips existing conflicting phrases.
pub struct StyledLine {
    // FIXME: Make this a &str with proper lifetime checking
    pub line: String,
    pub phrases: Vec<Phrase>,
}

// TODO: In the future when GATs are stable, we can implement IntoIterator.  Until then, users will
// just have to use self.phrases.iter() instead.
//
// impl IntoIterator for StyledLine<'a> {
//     type Item<'a> = StyledContent<&'a str>;
//     type IntoIter = std::vec::IntoIter<Self::Item>;
//     fn into_iter(self) -> Self::IntoIter {
//         self.phrases.into_iter()
//     }
// }

impl Phrase {
    fn new(start: usize, end: usize, patt: PattColor) -> Self {
        Self {
            start,
            end,
            patt,
        }
    }

    // Cleave a phrase in two. Returns a pair of left, right sides.
    fn partition(&self, pos: usize) -> (Option<Phrase>, Option<Phrase>) {
        if pos <= self.start {
            (None, Some(*self))
        } else if pos >= self.end() {
            (Some(*self), None)
        } else {
            let left = Self {end: pos, ..*self};
            let right = Self {start: pos, ..*self};
            (Some(left), Some(right))
        }
    }

    // Clip our phrase around other phrase. Returns optional new phrases for left and right unobscured parts.
    fn clip(&self, other: &Self) -> (Option<Phrase>, Option<Phrase>) {
        // Remove the overlapping range from self and return the non-overlapping range(s).
        // It is either 0, 1 or 2 phrases.

        let (left, _) = self.partition(other.start());
        let (_, right) = self.partition(other.end());
        (left, right)
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.start + self.len()
    }

    fn len(&self) -> usize {
        self.end - self.start
    }
}


impl StyledLine {
    pub fn new(line: &str, patt: PattColor) -> Self {
        Self {
            line: str::to_owned(line),
            phrases: vec![ Phrase::new(0, line.len(), patt) ],
        }
    }

    // fn to_str(&self) -> &str {
    //     for p in self.phrases {
    //         // FIXME: Impl this; use pattern instead of style in Phrase
    //         let style = to_style(p.style);
    //         &line[p.start, p.end];
    //         format!("{}" , style.apply(content))
    //     }
    // }


    // Inserts a new styled region into the line style planner.
    // If the new phrase overlaps with existing phrases, it clips them.
    pub fn push(&mut self, start: usize, end: usize, patt: PattColor) {
        assert!(end > start);
        let phrase = Phrase::new(start, end, patt);

        let insert_pos = self.phrases.binary_search_by_key(&start, |orig| orig.start);
        let (left, split_left)  = match insert_pos {
            Ok(pos) => {
                // The phrase at pos starts at the same position we do.  We will discard its left side.
                (pos, false)
            }
            Err(pos) => {
                // The phrase at pos-1 is clipped by us.  We will keep its left side.
                assert!(self.phrases.len() >= pos);
                assert!(pos > 0);
                (pos, true)
            }
        };

        // We want to insert our phrase at pos.
        // Find the phrase that starts after our end so we can decide if we need to insert or replace.

        // Rust bug?  This crashes:
        // let until_pos = self.phrases.binary_search_by_key(&end, |orig| orig.end);

        // let until_pos = self.phrases.binary_search_by_key(&end, |orig| orig.end);
        let count = self.phrases[left..].iter().take_while(|orig| orig.end < end).count();
        let split_right = self.phrases[left+count].end != end;
        // let (count, split_right) = match until_pos {
        //     Ok(until_pos) => {
        //         // The phrase at until_pos ends where we end.  Discard right side.
        //         (until_pos+1, false)
        //     }
        //     Err(until_pos) => {
        //         // The phrase at until_pos is clipped by us. We will keep its right side.
        //         assert!(until_pos + left <= self.phrases.len());
        //         (until_pos, true)
        //     }
        // };

        // let count = count - left;

        if count == 0 {
            // We are contained inside the phrase at pos and we split it into two pieces.
            // AAAAAAA
            //   BBB
            // CCCCCCC
            // DDD
            //     EEE
            if split_left {
                if split_right {
                    // BBB->
                    self.phrases.insert(left, Phrase { start: end, ..self.phrases[left-1]});
                }
                // <-BBB or <-EEE
                self.phrases[left-1].end = start;
            } else if split_right {
                // DDD->
                self.phrases[left].start = end;
            } else {
                // CCCCCCCCC
                self.phrases[left] = phrase;
            }
            if split_right || split_left {
                self.phrases.insert(left, phrase);
            }
        } else {
            // XXXXYYYY
            //   BBB
            // CCCCCCC
            // DDD
            //     EEE
            if split_left {
                self.phrases[left-1].end = phrase.start;
            }

            if count > 1 {
                // We can replace the existing phrase at left
                self.phrases[left] = phrase;

                // Remove the rest of the (count-1) phrases
                self.phrases.drain(left+1..left+count);
            } else {
                // We have to squeeze in between the two phrases we found
                self.phrases.insert(left, phrase);
            }
            if left + 1 < self.phrases.len() {
                self.phrases[left + 1].start = phrase.end;
            }
        }
    }
}


#[derive(Copy, Clone)]
pub enum PattColor {
    None,
    Normal,
    Highlight,
    Inverse,
    Timestamp,
    Pid(Color),
    Number(Color),
    Error,
    Fail,
    Info,
    NoCrumb,
    Module(Color),
}
/// Line section coloring
pub struct RegionColor {
    pub(crate) len: u16,
    pub(crate) style: PattColor,
}

fn to_style(patt: PattColor) -> ContentStyle {
    let style = ContentStyle::new();

    let style = match patt {
        PattColor::None => unreachable!("Tried to style with None pattern"),
        PattColor::Normal => style.reset(),
        PattColor::Highlight => style.with(Color::Yellow).on(Color::Blue).bold(),
        PattColor::Inverse => style.negative(),
        PattColor::Timestamp => style.with(Color::Green).on(Color::Black),
        PattColor::Pid(c) => style.with(c).on(Color::Black).italic(),
        PattColor::Number(c) => style.with(c).on(Color::Black),
        PattColor::Error => style.with(Color::Yellow).on(Color::Black),
        PattColor::Fail => style.with(Color::Red).on(Color::Blue).bold().italic(),
        PattColor::Info => style.with(Color::White).on(Color::Black),
        PattColor::NoCrumb => style.with(Color::White).on(Color::Black).italic(),
        PattColor::Module(c) => style.with(c).on(Color::Black).bold(),
    };
    style
}

impl RegionColor {
    pub(crate) fn to_str(&self, line: &str) -> String {
        let len = cmp::min(self.len as usize, line.len());
        let content = &line[..len];
        let style = to_style(self.style);

        format!("{}" , style.apply(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styledline_basic() {
        let mut line = StyledLine::new("hello", PattColor::Normal);
        assert!(line.phrases.len() == 1);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[0].end == 5);
    }


    #[test]
    fn test_styledline_overlap() {
        let line = "hello hello hello hello hello";
        let mut line = StyledLine::new(line, PattColor::Normal);

        // Overlap splits whole line
        line.push(10, 15, PattColor::Normal);
        assert!(line.phrases.len() == 3);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[0].end == 10);
        assert!(line.phrases[1].end == 15);
        assert!(line.phrases[2].end == 29);

        // Overlap aligns with start of existing
        line.push(0, 15, PattColor::Normal);

        assert!(line.phrases.len() == 2);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[0].end == 15);
        assert!(line.phrases[1].end == 29);

        // Overlap aligns with end of previous
        line.push(10, 15, PattColor::Normal);

        assert!(line.phrases.len() == 3);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[0].end == 10);
        assert!(line.phrases[1].end == 15);
        assert!(line.phrases[2].end == 29);

        // Overlap covers end of previous
        line.push(12, 20, PattColor::Normal);

        assert!(line.phrases.len() == 4);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[0].end == 10);
        assert!(line.phrases[1].end == 12);
        assert!(line.phrases[2].end == 20);
        assert!(line.phrases[3].end == 29);

        // Overlap covers multiple
        line.push(15, 20, PattColor::Normal);
        line.push(13, 25, PattColor::Normal);

        assert!(line.phrases.len() == 5);
        assert!(line.phrases[1].end == 12);
        assert!(line.phrases[2].end == 13);
        assert!(line.phrases[3].end == 25);

        line.push(0, 29, PattColor::Normal);
        assert!(line.phrases.len() == 1);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[0].end == 29);
    }

}
