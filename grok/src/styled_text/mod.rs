use crossterm::style::{Stylize, ContentStyle, StyledContent};
use std::{io, io::{stdout, Write}, cmp};
use crate::config::Config;
use lazy_static::lazy_static;
use regex::Regex;

use fnv::FnvHasher;
use std::hash::Hasher;
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

/// Holds a block of text as rows of stylable lines.
struct StyledText {
    lines : Vec<StyledLine>,
}

impl StyledText {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
        }
    }
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


    pub fn push(&mut self, start: usize, end: usize, patt: PattColor) {
        assert!(end > start);
        let phrase = Phrase::new(start, end, patt);
        let mut inserted = false;
        let phrases: Vec<Phrase> = self.phrases.iter()
            .map(|p| p.clip(&phrase))
            .fold(vec![], |mut acc, (l, r)| {
            if let Some(l) = l {
                acc.push(l);
            }
            if let Some(r) = r {
                if !inserted {
                    acc.push(phrase);
                    inserted = true;
                }
                acc.push(r);
            }
            acc
        });
        self.phrases = phrases;
        if !inserted {
            self.phrases.push(phrase);
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

pub fn to_style(patt: PattColor) -> ContentStyle {
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

pub struct ColorSequence {
    pub(crate) result: Vec<RegionColor>,
    pub(crate) default_style: PattColor,
    pub(crate) len: usize,
}

impl ColorSequence {
    pub(crate) fn new(default_style: PattColor) -> Self {
        Self {
            result: vec![],
            default_style,
            len: 0,
        }
    }

    pub(crate) fn push(&mut self, start: usize, end: usize, style: PattColor) -> usize {
        let last = self.len;
        assert!( start >= last );
        assert!( end >= start );

        if start > last {
            self.result.push(RegionColor {len: (start - last) as u16, style: self.default_style,});
        }
        if end > start {
            self.result.push(RegionColor { len: (end - start) as u16, style,});
        }
        self.len = end;
        end - last
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
    }

}
