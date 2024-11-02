use crossterm::terminal::ClearType;
use indexed_file::LineViewMode;
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, execute, queue, terminal};
use regex::Regex;

use crate::config::Config;
use crate::keyboard::UserCommand;
use crate::styled_text::{PattColor, RegionColor, StyledLine};
use crate::document::Document;
use crate::styled_text::RGB_BLACK;


#[derive(PartialEq, Debug)]
struct DisplayState {
    height: usize,
    width: usize,
}

struct ScreenBuffer {
    // content: String,
    content: Vec<StyledLine>,
    width: usize,
}

impl ScreenBuffer {

    fn new() -> Self {
        Self {
            content: Vec::new(),
            width: 0,
        }
    }

    fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    fn push(&mut self, line: StyledLine) {
        self.content.push(line)
    }

    fn push_raw(&mut self, data: &str) {
        self.content.push(StyledLine::new(data, PattColor::None))
    }
}

impl io::Write for ScreenBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.push_raw(s);
                Ok(s.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut buffer = String::new();
        for row in &self.content {
            let pairs = row.phrases.iter().zip(row.phrases[1..].iter());
            for (p, pnext) in pairs{
                match p.patt {
                    PattColor::None => {
                        buffer.push_str(&row.line);
                        break;
                    }
                    _ => {
                        let end = cmp::min(self.width, pnext.start);
                        assert!(end > p.start || end == 0);
                        let reg = RegionColor {len: (end - p.start) as u16, style: p.patt};
                        let content = reg.to_str(&row.line[p.start..end]);
                        buffer.push_str(content.as_str());
                        if end == self.width {
                            break;
                        }
                    }
                }
            }
        }
        let out = write!(stdout(), "{}", buffer);
        stdout().flush()?;
        self.content.clear();
        out
    }
}

enum ScrollAction {
    None,   // Nothing to do
    StartOfFile(usize),
    EndOfFile(usize),
    SearchForward,
    SearchBackward,
    Up(usize),
    Down(usize),
    Repaint,
    GotoPercent(f64),
    GotoOffset(usize),
}

pub struct Display {
    // Physical size of the display
    height: usize,
    width: usize,
    on_alt_screen: bool,

    use_alt: bool,
    color: bool,
    semantic_color: bool,


    /// Scroll command from user
    scroll: ScrollAction,

    /// Accumulated command argument
    arg_num: usize,
    arg_fraq: usize,
    arg_denom: usize,

    // Sticky whole-page scroll sizes
    whole: usize,

    // Sticky half-page scroll size
    half: usize,

    /// Size of the bottom status panel
    panel: usize,

    /// Previous display info
    prev: DisplayState,

    // Displayed line offsets
    displayed_lines: Vec<usize>,

    mouse_wheel_height: u16,

    mode: LineViewMode,

    search: Option<Regex>,
}

impl Drop for Display {
    fn drop(&mut self) {
        log::trace!("Display closing");
        self.stop().expect("Failed to stop display");
    }
}

impl Display {
    pub fn new(config: Config) -> Self {
        Self {
            height: 0,
            width: 0,
            on_alt_screen: false,
            use_alt: config.altscreen,
            scroll: ScrollAction::StartOfFile(0),
            arg_num: 0,
            panel: 1,
            whole: 0,
            half: 0,
            arg_fraq: 0,
            arg_denom: 0,
            prev: DisplayState { height: 0, width: 0},
            displayed_lines: Vec::new(),
            mouse_wheel_height: config.mouse_scroll,
            mode: LineViewMode::WholeLine,
            color: config.color,
            semantic_color: config.semantic_color,
            search: None,
        }
    }

    // Begin owning the terminal
    pub fn start(&mut self) -> crossterm::Result<()> {
        if ! self.on_alt_screen && self.use_alt {
            execute!(stdout(), terminal::EnterAlternateScreen)?;
            self.on_alt_screen = true;
        }

        // Hide the cursor
        execute!(stdout(), cursor::Hide)?;

        // Collect display size info
        self.update_size();

        Ok(())
    }

    fn stop(&mut self) -> crossterm::Result<()> {
        if self.on_alt_screen {
            execute!(stdout(), terminal::LeaveAlternateScreen).expect("Failed to exit alt mode");
            self.on_alt_screen = false;
            log::trace!("display: leave alt screen");
        }

        // Show the cursor
        execute!(stdout(), cursor::Show)?;

        Ok(())
    }

    fn update_size(&mut self) {
        let (width, height) = terminal::size().expect("Unable to get terminal size");
        self.width = width as usize;
        self.height = height as usize;

        // FIXME: Check config for Wrap mode
        self.mode = LineViewMode::Wrap{width: self.width};
    }

    fn page_size(&self) -> usize {
        cmp::max(self.height as isize - self.panel as isize, 0) as usize
    }

    fn set_status_msg(&mut self, _msg: String) {
        // FIXME
        // self.message = msg;
        // self.action = Action::Message;
    }

    pub fn set_search(&mut self, search: &str) -> bool {
        match Regex::new(search) {
            Ok(re) => { self.search = Some(re); true }
            Err(e) => {
                log::error!("Invalid search expression: {}", e);
                self.set_status_msg(format!("Invalid search expression: {}", e));
                false
            }
        }
    }

    pub fn set_filter(&mut self, doc: &mut Document, filter: &str) -> bool {
        match doc.set_filter(filter) {
            Ok(_) => true,
            Err(e) => {
                log::error!("Invalid filter expression: {}", e);
                self.set_status_msg(format!("Invalid filter expression: {}", e));
                false
            }
        }
    }

    // One line, or the given argument
    fn get_one(&self) -> usize {
        if self.arg_num > 0 {
            self.arg_num
        } else {
            1
        }
    }

    // Half-screen size, or the given argument
    fn get_half(&self) -> usize {
        if self.arg_num > 0 {
            self.arg_num
        } else if self.half > 0 {
            self.half
        } else {
            self.page_size() / 2
        }
    }

    // Whole-screen size, or the given argument
    fn get_whole(&self) -> usize {
        if self.arg_num > 0 {
            self.arg_num
        } else if self.whole > 0 {
            self.whole
        } else {
            self.page_size()
        }
    }

    // Sticky half-screen size
    fn sticky_half(&mut self) -> usize {
        if self.arg_num > 0 {
            self.half = self.arg_num;
        }
        self.get_half()
    }

    // Sticky whole-screen size
    fn sticky_whole(&mut self) -> usize {
        if self.arg_num > 0 {
            self.whole = self.arg_num;
        }
        self.get_whole()
    }

    fn collect_digit(&mut self, d: usize) {
        if self.arg_denom == 0 {
            // Mantissa
            self.arg_num = self.arg_num * 10 + d;
        } else {
            // Fraction
            self.arg_fraq = self.arg_fraq * 10 + d;
            self.arg_denom *= 10;
        }
    }

    fn collect_decimal(&mut self) {
        if self.arg_denom == 0 { self.arg_denom = 1; }
    }

    fn get_arg(&self) -> f64 {
        self.arg_num as f64 +
            if self.arg_denom > 0 {
                self.arg_fraq as f64 / self.arg_denom as f64
            } else {
                0f64
            }
    }

    pub fn handle_command(&mut self, cmd: UserCommand) {
        // FIXME: commands should be queued so we don't lose any. For example, search prompt needs us to refresh and search-next. So it
        //        calls us twice in a row.  I suppose we also need a way to cancel queued commands, then.  ^C? And some way to recognize
        //        commands that cancel previous ones (RefreshDisplay, twice in a row, for example).
        match cmd {
            UserCommand::ScrollDown => {
                self.scroll = ScrollAction::Down(self.get_one());
            }
            UserCommand::ScrollUp => {
                self.scroll = ScrollAction::Up(self.get_one());
            }
            UserCommand::CollectDigits(d) => {
                self.collect_digit(d as usize);
            }
            UserCommand::CollectDecimal => {
                self.collect_decimal();
            }
            UserCommand::PageDown => {
                self.scroll = ScrollAction::Down(self.get_whole());
            }
            UserCommand::PageUp => {
                self.scroll = ScrollAction::Up(self.get_whole());
            }
            UserCommand::PageDownSticky => {
                self.scroll = ScrollAction::Down(self.sticky_whole());
            }
            UserCommand::PageUpSticky => {
                self.scroll = ScrollAction::Up(self.sticky_whole());
            }
            UserCommand::HalfPageDown => {
                self.scroll = ScrollAction::Down(self.sticky_half());
            }
            UserCommand::HalfPageUp => {
                self.scroll = ScrollAction::Up(self.sticky_half());
            }
            UserCommand::ScrollToTop => {
                self.scroll = ScrollAction::StartOfFile(0);
            }
            UserCommand::ScrollToBottom => {
                self.scroll = ScrollAction::EndOfFile(0);
            }
            UserCommand::SeekStartLine => {
                self.scroll = ScrollAction::StartOfFile(self.get_arg() as usize);
            }
            UserCommand::SeekEndLine => {
                self.scroll = ScrollAction::EndOfFile(self.get_arg() as usize);
            }
            UserCommand::RefreshDisplay => {
                self.scroll = ScrollAction::Repaint;
            }
            UserCommand::GotoPercent => {
                self.scroll = ScrollAction::GotoPercent(self.get_arg())
            }
            UserCommand::GotoOffset => {
                self.scroll = ScrollAction::GotoOffset(self.get_arg() as usize)
            }
            UserCommand::TerminalResize => {
                self.update_size();
            }
            UserCommand::SelectWordAt(_x, _y) => {
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::SelectWordDrag(_x, _y) => {
                // println!("{:?}\r", cmd);
                // FIXME: Highlight the words selected
                // Add to some search struct and highlight matches
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::MouseScrollUp => {
                self.scroll = ScrollAction::Up(self.mouse_wheel_height as usize);
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::MouseScrollDown => {
                self.scroll = ScrollAction::Down(self.mouse_wheel_height as usize);
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::SearchNext => {
                self.scroll = ScrollAction::SearchForward;
            }
            UserCommand::SearchPrev => {
                self.scroll = ScrollAction::SearchBackward;
            }
            _ => {}
        }

        // Clear argument when command is seen
        if ! matches!(self.scroll, ScrollAction::None) {
            self.arg_num = 0;
            self.arg_denom = 0;
            self.arg_fraq = 0;
        }
    }

    fn draw_styled_line(&self, buff: &mut ScreenBuffer, row: usize, line: StyledLine) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.set_width(self.width);
        buff.push(line);

        queue!(buff, crossterm::style::SetBackgroundColor(RGB_BLACK), terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_line(&self, doc: &Document, buff: &mut ScreenBuffer, row: usize, line: &str) {
        if self.color && self.semantic_color {
            // TODO: Memoize the line_colors along with the lines
            self.draw_styled_line(buff, row, doc.line_colors(line));
        } else {
            self.draw_plain_line(doc, buff, row, line);
        }
    }

    fn draw_plain_line(&self, _doc: &Document, buff: &mut ScreenBuffer, row: usize, line: &str) {
        // TODO: dedup with draw_styled_line (it only needs to remove the RGB_BLACK background)
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.set_width(self.width);
        buff.push(StyledLine::sanitize_basic(line, PattColor::Plain));

        queue!(buff, terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }
}

#[derive(Debug)]
struct ScrollVector {
    offset: usize,  // byte offset in Document
    lines: usize,   // number of lines to display
}

#[derive(Debug)]
enum Scroll {
    Up(ScrollVector),
    Down(ScrollVector),
    Repaint(ScrollVector),
    GotoTop(ScrollVector),
    GotoBottom(ScrollVector),
    None,
}

impl Scroll {
    fn down(offset: usize, lines: usize) -> Self {
        Self::Down( ScrollVector {offset, lines} )
    }
    fn up(offset: usize, lines: usize) -> Self {
        Self::Up( ScrollVector {offset, lines} )
    }
    fn repaint(offset: usize, lines: usize) -> Self {
        Self::Repaint( ScrollVector {offset, lines} )
    }
    fn goto_top(offset: usize, lines: usize) -> Self {
        Self::GotoTop( ScrollVector {offset, lines} )
    }
    fn goto_bottom(offset: usize, lines: usize) -> Self {
        Self::GotoBottom( ScrollVector {offset, lines} )
    }
    fn none() -> Self {
        Self::None
    }
    fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl Display {
    // Pull lines from an iterator and display them.  There are three modes:
    // 1. Scroll up:  Display each new line at the next lower position, and scroll up from bottom
    // 2. Scroll down:  Display each new line at the next higher position, and scroll down from top
    // 3. Repaint:  Display all lines from the given offset
    // pos is the offset in the file for the first line
    // Scroll distance is in screen rows.  If a read line takes multiple rows, they count as multiple lines.
    fn feed_lines(&mut self, doc: &mut Document, mode: LineViewMode, scroll: Scroll) -> crossterm::Result<ScreenBuffer> {
        log::trace!("feed_lines: {:?}", scroll);

        let mut buff = ScreenBuffer::new();

        let top_of_screen = 0;
        let height = self.page_size();

        let (lines, mut row, mut count) = match scroll {
            Scroll::Up(sv) | Scroll::GotoBottom(sv) => {
                // Partial or complete screen scroll backwards
                let skip = sv.lines.saturating_sub(height);
                let lines: Vec<_> = doc.get_lines_from_rev(mode, sv.offset, sv.lines).into_iter().skip(skip).rev().collect();
                let rows = lines.len();
                queue!(buff, terminal::ScrollDown(rows as u16)).unwrap();
                self.displayed_lines.splice(0..0, lines.iter().map(|(pos, _)| *pos).take(rows));
                self.displayed_lines.truncate(height);
                // TODO: add test for whole-screen offsets == self.displayed_lines
                (lines, 0, 0)
            },
            Scroll::Down(sv) => {
                // Partial screen scroll forwards
                let skip = sv.lines.saturating_sub(height);
                let mut lines = doc.get_lines_from(mode, sv.offset, sv.lines + 1);
                if !lines.is_empty() {
                    // TODO: Only if scrolling down from some pos; not when homing: assert_eq!(lines.first().unwrap().0, sv.offset);
                    lines = lines.into_iter().skip(skip + 1).collect();
                }
                let rows = lines.len();
                queue!(buff, terminal::ScrollUp(rows as u16)).unwrap();
                self.displayed_lines = if self.displayed_lines.len() > rows {
                    self.displayed_lines[rows..].to_vec()
                } else {
                    Vec::new()
                };
                self.displayed_lines.extend(lines.iter().map(|(pos, _)| *pos).take(rows));
                (lines, height - rows, 0)
            },
            Scroll::Repaint(sv) | Scroll::GotoTop(sv) => {
                // Repainting whole screen, no scrolling
                let lines = doc.get_lines_from(mode, sv.offset, sv.lines.min(height));
                let skip = sv.lines.saturating_sub(height);
                let lines:Vec<_> = lines.into_iter().skip(skip).collect();
                let rows = lines.len();
                // queue!(buff, terminal::Clear(ClearType::All)).unwrap();
                self.displayed_lines = lines.iter().map(|(pos, _)| *pos).take(rows).collect();
                (lines, 0, height)
            },
            Scroll::None => unreachable!("Scroll::None")
        };

        for (pos, line) in lines.iter(){
            assert_eq!(self.displayed_lines[row - top_of_screen],  *pos);
            self.draw_line(doc, &mut buff, row, line);
            row += 1;
            count = count.saturating_sub(1);
        }

        while count > 0 && row < height {
            // TODO: special color for these
            self.draw_line(doc, &mut buff, row, "~");
            row += 1;
            count = count.saturating_sub(1);
        }

        Ok(buff)
    }

    pub fn refresh_screen(&mut self, doc: &mut Document) -> crossterm::Result<()> {
        // FIXME: Discard unused cached lines

        let view_height = self.page_size();

        // Our new display
        let disp = DisplayState {
            height: self.page_size(),
            width: self.width,
        };

        let plan =
            if self.displayed_lines.is_empty() {
                // Blank slate; start of file
                log::trace!("start of file");
                Scroll::repaint(0, view_height)
            } else if disp != self.prev {
                // New screen dimensions; repaint everything
                // FIXME: No need to repaint if we got vertically smaller
                // FIXME: Only need to add rows if we only got taller
                log::trace!("repaint everything");
                Scroll::repaint(*self.displayed_lines.first().unwrap(), view_height)
            } else {
                match self.scroll {
                    ScrollAction::GotoOffset(offset) => {
                        // Scroll to the given offset
                        log::trace!("scroll to offset {}", offset);
                        Scroll::goto_top(offset, view_height)
                    }
                    ScrollAction::GotoPercent(percent) => {
                        // Scroll to the given percentage of the document
                        log::trace!("scroll to percent {}", percent);
                        let offset = doc.len() as f64 * percent / 100.0;
                        Scroll::goto_top(offset as usize, view_height)
                    }
                    ScrollAction::Repaint => {
                        log::trace!("repaint everything");
                        Scroll::repaint(*self.displayed_lines.first().unwrap(), view_height)
                    }
                    ScrollAction::StartOfFile(line) => {
                        // Scroll to top
                        log::trace!("scroll to top");
                        Scroll::goto_top(0, view_height + line.saturating_sub(1))
                    }
                    ScrollAction::EndOfFile(line) => {
                        // Scroll to bottom
                        log::trace!("scroll to bottom");
                        Scroll::goto_bottom(usize::MAX, view_height + line.saturating_sub(1))
                    }
                    ScrollAction::Up(len) => {
                        // Scroll up 'len' lines before the top line
                        log::trace!("scroll up {} lines", len);
                        let begin = self.displayed_lines.first().unwrap();
                        Scroll::up(*begin, len)
                    }
                    ScrollAction::Down(len) => {
                        // Scroll down 'len' lines after the last line displayed
                        log::trace!("scroll down {} lines", len);
                        let begin = self.displayed_lines.last().unwrap();
                        Scroll::down(*begin, len)
                    }
                    ScrollAction::SearchBackward => {
                        // Search backwards from the first line displayed
                        log::trace!("search backward");
                        // todo!("Tell doc to search backwards");
                        // Need to search backwards through the document until we find a match.
                        // If no match, need to cancel the action.
                        // If user cancels, need to cancel the action.
                        // Create a FilterIndex(doc) to build the search index.
                        let begin = self.displayed_lines.first().unwrap();
                        Scroll::up(*begin, view_height)
                    }
                    ScrollAction::SearchForward => {
                        // Search forwards from the last line displayed
                        log::trace!("search forward");
                        let begin = self.displayed_lines.last().unwrap();
                        Scroll::down(*begin, view_height)
                    }
                    ScrollAction::None => Scroll::none()
                }
            };

        self.scroll = ScrollAction::None;

        if plan.is_none() {
            return Ok(());
        }

        log::trace!("screen changed");

        let mut buff = self.feed_lines(doc, self.mode, plan)?;
        self.prev = disp;

        // DEBUG HACK
        // self.draw_line(doc, &mut buff, self.height - 2, &format!("scroll={} displayed={:?}", scroll, self.displayed_lines));
        buff.flush()
    }

}