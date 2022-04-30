use crossterm::style::{Stylize, ContentStyle, StyledContent};
use crate::config::Config;
use lazy_static::lazy_static;
use regex::Regex;

use fnv::FnvHasher;
use std::hash::Hasher;

pub trait Stylable {
    fn stylize(&mut self, row: &str) -> ();
}

/// Defines a style for a portion of a line.  Represents the style and the position within the line.
/// The line content is included here for easier iteration, but the whole line lives elsewhere.
#[derive(Copy, Clone)]
struct Phrase<'a> {
    start: usize,
    phrase: StyledContent<&'a str>,
}

/// Holds a line of text and the styles for each character.
/// The styles are stored in phrases, a sorted collection of start,end,style.
/// Phrases are not allowed to overlap. If an overlapping phrase is added, it clips existing conflicting phrases.
struct StyledLine<'a> {
    line: &'a str,
    phrases: Vec<Phrase<'a>>,
}

/// Holds a block of text as rows of stylable lines.
struct StyledText<'a> {
    lines : Vec<StyledLine<'a>>,
}

impl StyledText<'_> {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
        }
    }
}

impl<'a> StyledLine<'a> {
    fn new(line: &'a str, style: ContentStyle) -> Self {
        Self {
            line,
            phrases: vec![ Phrase::new(0, line.len(), style, line) ],
        }
    }
}

impl<'a> Phrase<'a> {
    fn new(start: usize, end: usize, style: ContentStyle, line: &'a str) -> Self {
        Self {
            start,
            phrase: StyledContent::new(style, &line[start..end]),
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

impl<'a> Phrase<'a> {

    // Cleave a phrase in two. Returns a pair of left, right sides.
    fn partition(&'a self, pos: usize) -> (Option<Phrase<'a>>, Option<Phrase<'a>>) {
        if pos <= self.start {
            (None, Some(*self))
        } else if pos >= self.end() {
            (Some(*self), None)
        } else {
            let x = pos - self.start;
            assert!(x < self.len());
            assert!(x > 0);

            let content = self.content();

            let left = Self {
                start: self.start,
                phrase: StyledContent::new(*self.style(), &content[0..x]),
            };

            let right = Self {
                start: self.start + x,
                phrase: StyledContent::new(*self.style(), &content[x..]),
            };

            (Some(left), Some(right))
        }
    }

    // Clip our phrase around other phrase. Returns optional new phrases for left and right unobscured parts.
    fn clip(&'a self, other: &Self) -> (Option<Phrase<'a>>, Option<Phrase<'a>>) {
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
        self.len()
    }

    fn style(&self) -> &ContentStyle {
        self.phrase.style()
    }

    fn content(&self) -> &str {
        self.phrase.content()
    }
}


impl<'a> StyledLine<'a> {
    fn push(&'a mut self, start: usize, end: usize, style: ContentStyle) {
        assert!(end > start);
        let phrase = Phrase::new(start, end, style, self.line);
        let mut inserted = false;
        {
            let phrases: Vec<Phrase<'a>> = self.phrases.iter()
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
        }
        // self.phrases = phrases;
        if !inserted {
            self.phrases.push(phrase);
        }
    }
}