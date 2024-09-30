use crossterm::style::{Stylize, ContentStyle};
use std::cmp;
use crossterm::style::Color;

/// Defines a style for a portion of a line.  Represents the style and the position within the line.
#[derive(Copy, Clone)]
pub struct Phrase {
    pub start: usize,
    pub patt: PattColor,
}

/// Holds a line of text and the styles for each character.
/// The styles are stored in phrases, a sorted collection of start,style.
/// Phrases are not allowed to overlap. When a phrase is inserted that overlaps an existing one,
/// the existing one is clipped to fit around the new one.
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
    fn new(start: usize, patt: PattColor) -> Self {
        Self {
            start,
            patt,
        }
    }
}


impl StyledLine {
    pub fn new(line: &str, patt: PattColor) -> Self {
        Self {
            line: str::to_owned(line),
            phrases: vec![ Phrase::new(0, patt), Phrase::new(line.len(), patt), ],
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
        let phrase = Phrase::new(start, patt);

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
        let until_pos = self.phrases.binary_search_by_key(&end, |orig| orig.start);
        let (right, split_right) = match until_pos {
            Ok(until_pos) => {
                // The phrase at until_pos ends where we end.  Discard right side.
                (until_pos, false)
            }
            Err(until_pos) => {
                // The phrase before until_pos is clipped by us. We will keep its right side.
                assert!(until_pos > 0);
                (until_pos, true)
            }
        };

        let count = right - left;


        // We may be contained inside the phrase at pos and we need to split it into two pieces.
        // AAAAAAA
        //   BBB      split_left && split_right:   Insert copy of AA; insert our new phrase
        // CCCCCCC    !split_left && !split_right: replace CCCCCCCC with our phrase
        // DDD        split_right && count=1:      Insert our new phrase at left; adjust left+1 to end
        //     EEE    split_left && count=0:       Insert our new phrase at left; adjust left-1 to end
        if count == 0 && split_left && split_right {
            // BBB->  Insert copy of AA
            self.phrases.insert(left, Phrase { start: end, ..self.phrases[left-1]});
        }
        if count < 2 && (split_left || split_right) {
            // <-BBB || DDD-> || <-EEE We have to squeeze in between the two phrases we found
            self.phrases.insert(left, phrase);
        } else {
            assert!(count > 0);
            // CCCCCCC
            // We can replace the existing phrase at left
            self.phrases[left] = phrase;

            // Remove the rest of the (count-1) phrases
            if split_right {
                self.phrases.drain(left+1..right-1);
            } else {
                self.phrases.drain(left+1..right);
            }
        }
        assert!(left + 1 < self.phrases.len());
        self.phrases[left + 1].start = end;
    }
}

pub static RGB_BLACK: Color = Color::Rgb{r:0,g:0,b:0};

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
        PattColor::Normal => style.with(Color::Green).on(RGB_BLACK),
        PattColor::Highlight => style.with(Color::Yellow).on(Color::Blue).bold(),
        PattColor::Inverse => style.negative(),
        PattColor::Timestamp => style.with(Color::Green).on(RGB_BLACK),
        PattColor::Pid(c) => style.with(c).on(RGB_BLACK).italic(),
        PattColor::Number(c) => style.with(c).on(RGB_BLACK),
        PattColor::Error => style.with(Color::Yellow).on(RGB_BLACK),
        PattColor::Fail => style.with(Color::Red).on(Color::Blue).bold().italic(),
        PattColor::Info => style.with(Color::White).on(RGB_BLACK),
        PattColor::NoCrumb => style.with(Color::White).on(RGB_BLACK).italic(),
        PattColor::Module(c) => style.with(c).on(RGB_BLACK).bold(),
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
        let line = StyledLine::new("hello", PattColor::Normal);
        assert!(line.phrases.len() == 2);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[1].start == 5);
    }


    #[test]
    fn test_styledline_overlap() {
        let line = "hello hello hello hello hello";
        let mut line = StyledLine::new(line, PattColor::Normal);

        // Overlap splits whole line
        line.push(10, 15, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 10, 15, 29]);

        // Overlap aligns with start of existing
        line.push(0, 15, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 15, 29]);

        // Patter replaces one existing pattern exactly
        line.push(0, 15, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 15, 29]);

        // Overlap aligns with end of previous
        line.push(10, 15, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 10, 15, 29]);

        // Overlap covers end of previous
        line.push(12, 20, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 10, 12, 20, 29]);

        // Overlap covers multiple
        line.push(15, 20, PattColor::Normal);
        line.push(13, 25, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 10, 12, 13, 25, 29]);

        // Left-aligned, right-disjoint, not at end
        line.push(13, 20, PattColor::Normal);
        assert_eq!(line.phrases.iter().map(|p| p.start).collect::<Vec<_>>(), vec![0, 10, 12, 13, 20, 25, 29]);

        line.push(0, 29, PattColor::Normal);
        assert!(line.phrases.len() == 2);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[1].start == 29);
    }

}
