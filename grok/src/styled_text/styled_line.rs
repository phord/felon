use crossterm::style::{Stylize, ContentStyle};
use fnv::FnvHasher;
use itertools::Itertools;
use regex::Regex;
use std::{hash::Hasher, ops::Range};
use crossterm::style::Color;

/// Defines a style for a portion of a line.  Represents the style and the position within the line.
#[derive(Copy, Clone, Debug)]
struct Phrase {
    start: usize,
    patt: PattColor,
}

/// Holds a line of text and the styles for each character.
/// The styles are stored in phrases, a sorted collection of start,style.
/// Phrases are not allowed to overlap. When a phrase is inserted that overlaps an existing one,
/// the existing one is clipped to fit around the new one.
#[derive(Clone)]
pub struct StyledLine {
    // FIXME: Make this a &str with proper lifetime checking
    pub line: String,
    phrases: Vec<Phrase>,
}

impl Phrase {
    fn new(start: usize, patt: PattColor) -> Self {
        Self {
            start,
            patt,
        }
    }
}

const TAB_SIZE: usize = 8;

// https://stackoverflow.com/questions/51982999/slice-a-string-containing-unicode-chars
pub fn utf8_slice(s: &str, start: usize, end: usize) -> Option<&str> {
    let mut iter = s.char_indices()
        .map(|(pos, _)| pos)
        .chain(Some(s.len()))
        .skip(start)
        .peekable();
    let start_pos = *iter.peek()?;
    for _ in start..end { iter.next(); }
    Some(&s[start_pos..*iter.peek()?])
}


impl StyledLine {
    pub fn new(line: &str, patt: PattColor) -> Self {
        // Init a line with a start phrase and an end phrase
        Self {
            line: str::to_owned(line),
            phrases: vec![ Phrase::new(0, patt), Phrase::new(line.len(), patt), ],
        }
    }

    pub fn apply(&mut self, match_string: &str, range: Range<usize>, patt: PattColor) {
        let pattern = match patt {
            PattColor::None => return,  // No pattern to apply
            PattColor::Semantic =>  PattColor::Number(Self::hash_color(match_string)),
            _ => patt,
        };
        self.push(range.start, range.end, pattern);
    }

    pub(crate) fn default_santize_regex() -> regex::Regex {
        Regex::new(r"[\x00-\x08\x0A-\x1f\x7f-\xff]").unwrap()
    }

    pub fn sanitize_basic(&mut self) {
        // TODO Replace this with a Stylist::Replace action
        let mut out = String::with_capacity(self.line.len());
        let mut it_phrases = self.phrases.iter_mut();
        let mut next_phrase = it_phrases.next().unwrap();
        let mut offset = 0;

        for ch in self.line.chars() {
            while offset == next_phrase.start {
                next_phrase.start = out.len();
                next_phrase = it_phrases.next().unwrap();
            }
            offset += ch.len_utf8();
            match ch {
                // '\r' | // TODO: allow \r delimited lines? Filter out only \r\n?  For now, show ^M
                '\n' => { continue },
                '\t' => {
                    let start = out.len();
                    let len = TAB_SIZE - start % TAB_SIZE;
                    out.push_str(&" ".repeat(len));
                },
                '\x00'..='\x1f' | '\u{7f}'..='\u{FF}'=> {
                    match ch {
                        '\x1b' => out.push_str("ESC"),
                        '\x00'..='\x1f' => { out.push('^'); out.push((b'@' + ch as u8) as char); },
                        '\x7f' => out.push_str("^?"),
                        '\u{80}'..='\u{FF}' => out.push_str(format!("<{:#X}>", ch as u8).as_str()),
                        _ => unreachable!("Outer pattern mismatch: {:?}", ch),
                    }
                },
                _ => out.push(ch),
            }
        }
        // Clean up any stragglers
        for next_phrase in it_phrases {
            assert!(next_phrase.start <= out.len(), "unexpected gap before phrase: {:?} {} {}", next_phrase, out.len(), out);
            next_phrase.start = out.len();
        }
        self.line = out;
    }

    pub fn to_string(&self, start: usize, width: usize) -> String {
        let end = self.line.len().min(start + width);
        self.phrases.windows(2)
            .map(|phrases| (phrases[0], phrases[1]))
            .filter(|(p, pnext)| p.start < end && pnext.start > start && p.start < pnext.start)
            .map(|(p, pnext)| {
                match p.patt {
                    PattColor::None => {  // None: No patterns for whole line
                        utf8_slice(&self.line, start, end).unwrap().to_string()
                    }
                    _ => {
                        let start = start.max(p.start);
                        let end = end.min(pnext.start);
                        let reg = RegionColor {style: p.patt};
                        if let Some(slice) = utf8_slice(&self.line, start, end) {
                            reg.to_str(slice)
                        } else {
                            "".to_string()
                        }
                    }
                }
        })
        .join("")
    }

    fn hash_color(text: &str) -> Color {
        let mut hasher = FnvHasher::default();
        hasher.write(text.as_bytes());
        let hash = hasher.finish();

        let base = 0x80_u8;
        let red = (hash & 0xFF) as u8 | base;
        let green = ((hash >> 8) & 0xFF) as u8 | base;
        let blue = ((hash >> 16) & 0xFF) as u8 | base;

        Color::Rgb {r: red, g: green, b: blue}
    }

    // Inserts a new styled region into the line style planner.
    // If the new phrase overlaps with existing phrases, it clips the existing ones.
    pub fn push(&mut self, start: usize, end: usize, patt: PattColor) {
        if end == start { return }
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
        // AAAAAAA    <--- This phrase exists ---  What happens when we insert the next one
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
#[derive(Debug)]
#[allow(dead_code)]
pub enum PattColor {
    None,       // No pattern possible for this line

    Plain,      // Use default terminal colors
    Normal,
    Semantic,
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
    pub(crate) style: PattColor,
}

fn to_style(patt: PattColor) -> ContentStyle {
    let style = ContentStyle::new();

    match patt {
        PattColor::None => unreachable!("Tried to style with None pattern"),
        PattColor::Plain => style,
        PattColor::Normal => style.with(Color::Green).on(RGB_BLACK),
        PattColor::Semantic => panic!("Semantic colors should be pre-processed"),
        PattColor::Highlight => style.with(Color::Yellow).on(Color::Blue).bold(),
        PattColor::Inverse => style.negative(),
        PattColor::Timestamp => style.with(Color::Green).on(RGB_BLACK),
        PattColor::Pid(c) => style.with(c).on(RGB_BLACK).italic(),
        PattColor::Number(c) => style.with(c).on(RGB_BLACK),
        PattColor::Error => style.with(Color::Yellow).on(RGB_BLACK),
        PattColor::Fail => style.with(Color::Red).on(Color::Blue).bold().italic(),
        PattColor::Info => style.with(Color::White).on(RGB_BLACK),
        PattColor::NoCrumb => style.with(Color::White).on(RGB_BLACK), // .italic(),
        PattColor::Module(c) => style.with(c).on(RGB_BLACK).bold(),
    }
}

impl RegionColor {
    pub(crate) fn to_str(&self, line: &str) -> String {
        let style = to_style(self.style);
        format!("{}" , style.apply(line))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crossterm::{style::{Attribute, Print, SetAttribute}, QueueableCommand};

    use super::*;

    #[test]
    fn test_styledline_basic() {
        let line = StyledLine::new("hello", PattColor::Normal);
        assert!(line.phrases.len() == 2);
        assert!(line.phrases[0].start == 0);
        assert!(line.phrases[1].start == 5);
    }

    #[test]
    fn crossterm_style_test() {
        // use crossterm::style::Stylize;

        crossterm::terminal::enable_raw_mode().expect("can enable raw mode");

        // println!("{}", "Blue text".blue());
        // println!("{}", "Negative text".negative());

        let mut stdout = std::io::stdout();

        stdout.queue(Print("\r\n".to_string())).expect("can queue");  // Clear line
        stdout.queue(SetAttribute(Attribute::Undercurled)).expect("can queue");
        // stdout.queue(SetAttribute(Attribute::CrossedOut)).expect("can queue");
        // stdout.queue(SetAttribute(Attribute::CrossedOut)).expect("can queue");
        // stdout.queue(SetAttribute(Attribute::Underdashed)).expect("can queue");
        // stdout.queue(SetAttribute(Attribute::Underdotted)).expect("can queue");
        stdout.queue(Print("REVERSED TEXT")).expect("can queue");
        stdout.queue(SetAttribute(Attribute::Reset)).expect("can queue");
        stdout.queue(Print("\r\n".to_string())).expect("can queue");
        stdout.flush().expect("can flush");

        // Don't forget to cleanup
        crossterm::terminal::disable_raw_mode().expect("can disable raw mode");

        println!("{}", "Bold text".bold());
        println!("{}", "Underlined text".underlined());
        println!("{}", "Underlined text fail".underline(Color::Red)); // .attribute(Attribute::Underdashed)

        // Sleep a bit to see the output
        // std::thread::sleep(std::time::Duration::from_secs(1));

        // panic!("Forcing output");
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

        // Pattern replaces one existing pattern exactly
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

    // FIXME: Unit tests for StyledLine::to_string

}
