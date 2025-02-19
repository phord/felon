use crossterm::terminal::ClearType;
use indexed_file::LogLine;
use std::{cmp, io::{self, stdout, Write}};
use crossterm::{cursor, execute, queue, terminal};

use crate::{config::Config, styled_text::LineViewMode};
use crate::keyboard::UserCommand;
use crate::styled_text::styled_line::{StyledLine, RGB_BLACK};
use crate::document::Document;


#[derive(PartialEq, Debug)]
struct DisplayState {
    height: usize,
    width: usize,
    pan: usize,
}

struct ScreenBuffer {
    content: String,
}

impl ScreenBuffer {

    fn new() -> Self {
        Self {
            content: String::new(),
        }
    }

    fn push_raw(&mut self, data: &str) {
        self.content.push_str(data)
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
        let out = write!(stdout(), "{}", self.content);
        stdout().flush()?;
        self.content.clear();
        out
    }
}

enum ScrollAction {
    None,   // Nothing to do
    StartOfFile(usize),
    EndOfFile(usize),
    Search(bool, usize),
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
    chop: bool,

    /// Right-scroll pan position
    pan: usize,

    // Sticky pan width
    pan_width: usize,

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

    // Search direction
    search_forward: bool,

    mouse_wheel_height: u16,
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
            prev: DisplayState { height: 0, width: 0, pan: 0},
            displayed_lines: Vec::new(),
            mouse_wheel_height: config.mouse_scroll,
            color: config.color,
            chop: config.chop,
            pan: 0,
            search_forward: true,
            pan_width: 0,
        }
    }

    // Begin owning the terminal
    pub fn start(&mut self) -> std::io::Result<()> {
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

    fn stop(&mut self) -> std::io::Result<()> {
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
    }

    fn page_size(&self) -> usize {
        cmp::max(self.height as isize - self.panel as isize, 0) as usize
    }

    fn set_status_msg(&mut self, _msg: String) {
        // FIXME
        // self.message = msg;
        // self.action = Action::Message;
    }

    pub fn set_search(&mut self, doc: &mut Document, search: &str, forward: bool) -> bool {
        self.search_forward = forward;
        match doc.set_search(search) {
            Ok(_) => true,
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

    // Half screen width, or sticky previous value, or given argument
    fn get_pan_width(&mut self) -> usize {
        if self.arg_num > 0 {
            self.pan_width = self.arg_num;
            self.arg_num
        } else if self.pan_width > 0 {
            self.pan_width
        } else {
            self.width / 2
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

    /// Direct jump to some location because a previous op completed
    pub fn goto(&mut self, offset: usize) {
        self.scroll = ScrollAction::GotoOffset(offset);
    }

    pub fn handle_command(&mut self, cmd: UserCommand) {
        // FIXME: commands should be queued so we don't lose any. For example, search prompt needs us to refresh and search-next. So it
        //        calls us twice in a row.  I suppose we also need a way to cancel queued commands, then.  ^C? And some way to recognize
        //        commands that cancel previous ones (RefreshDisplay, twice in a row, for example).
        match cmd {
            UserCommand::PanLeft => {
                self.pan = self.pan.saturating_sub(self.get_pan_width());
                // self.scroll = ScrollAction::Repaint;
            }
            UserCommand::PanRight => {
                self.pan += self.get_pan_width();
                // self.scroll = ScrollAction::Repaint;
            }
            UserCommand::PanLeftMax => {
                self.pan = 0;
                // self.scroll = ScrollAction::Repaint;
            }
            UserCommand::PanRightMax => {
                // FIXME: Magic number that means "pan to EOL"
                self.pan = usize::MAX;
                // self.scroll = ScrollAction::Repaint;
            }
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
                // self.scroll = ScrollAction::Repaint;
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
                self.scroll = ScrollAction::Search(self.search_forward, self.get_arg() as usize);
            }
            UserCommand::SearchPrev => {
                self.scroll = ScrollAction::Search(!self.search_forward, self.get_arg() as usize);
            }
            _ => {}
        }

        // Clear argument when any user action but digits/decimal is seen
        if ! matches!(cmd, UserCommand::CollectDigits(_) | UserCommand::CollectDecimal | UserCommand::TerminalResize) {
            self.arg_num = 0;
            self.arg_denom = 0;
            self.arg_fraq = 0;
        }
    }

    fn draw_styled_line(&self, buff: &mut ScreenBuffer, row: usize, line: StyledLine) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.push_raw(line.to_string(0, self.width).as_str());

        // FIXME: Push this into Stylist somehow
        queue!(buff, crossterm::style::SetBackgroundColor(RGB_BLACK), terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_log_line(&self, buff: &mut ScreenBuffer, row: usize, line: &LogLine) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        // Used for LogLines that are already rendered with Stylist.  TODO: New type? StyledLogLine?
        buff.push_raw(line.line.as_str());

        // FIXME: Push this into Stylist somehow
        queue!(buff, crossterm::style::SetBackgroundColor(RGB_BLACK), terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_line(&self, doc: &Document, buff: &mut ScreenBuffer, row: usize, line: &str) {
        if self.color {
            self.draw_styled_line(buff, row, doc.line_colors(line));
        } else {
            self.draw_plain_line(doc, buff, row, line);
        }
    }

    fn draw_plain_line(&self, _doc: &Document, buff: &mut ScreenBuffer, row: usize, line: &str) {
        // TODO: dedup with draw_styled_line (it only needs to remove the RGB_BLACK background)
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.push_raw(line);

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
    Overwrite(usize/* start row */, ScrollVector),
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
    fn overwrite(offset: usize, start: usize, end: usize) -> Self {
        Self::Overwrite( start, ScrollVector {offset,lines: end - start} )
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

    fn paint(&mut self, doc: &mut Document, lines: Vec<indexed_file::LogLine>, scroll: Scroll) -> std::io::Result<ScreenBuffer> {
        let mut buff = ScreenBuffer::new();
        let height = self.page_size();

        // Optimize when repainting the whole screen
        let repaint = lines.len() == height;

        let (mut row, incr, count) = if repaint {
            (0, 1, 0)
        } else {
            match scroll {
                Scroll::Up(_) => (0, 0, 0),
                Scroll::Down(_) => (height - 1, 0, 0),
                Scroll::GotoBottom(_) | Scroll::GotoTop(_) | Scroll::Repaint(_) => (0, 1, height),
                Scroll::Overwrite(start, _) => (start, 1, lines.len()),
                Scroll::None => unreachable!("Scroll::None")
            }
        };

        // Ugh! This is confusing.  But if we're Scrolling Up (towards the top of the document) then we're going to
        // move the lines on the screen already downward.  So if scrolling up, we slide the lines on screen down.

        // Scrolling the screen down if we are scrolling the document up
        let down = !repaint && matches!(scroll, Scroll::Up(_));

        // Scrolling the screen up if we are only inserting at the bottom
        let up = !down && incr == 0;

        let reversed = lines.len() > 1 && lines[0].offset > lines[1].offset;

        let mut iter = lines.iter();
        let iter: &mut dyn Iterator<Item = &LogLine> =
            if reversed && !down {
                // reverse the lines because we decided not to scroll
                &mut iter.rev()
            } else {
                &mut iter
            };

        let filler = count.saturating_sub(lines.len());
        for line in iter {
            if down {
                queue!(buff, terminal::ScrollDown(1)).unwrap();
            } else if up {
                queue!(buff, terminal::ScrollUp(1)).unwrap();
            }
            self.draw_log_line(&mut buff, row, line);
            row += incr;
        }

        for _ in 0..filler {
            if down {
                queue!(buff, terminal::ScrollDown(1)).unwrap();
            } else if up {
                queue!(buff, terminal::ScrollUp(1)).unwrap();
            }
            self.draw_line(doc, &mut buff, row, "~");
            row += incr;
        }
        assert!(filler <= height);

        // Record the displayed offsets
        let offsets = lines.iter().map(|logline| logline.offset);
        if down {
            // If we scrolled down, insert the offsets in reverse at the start of the list
            self.displayed_lines.splice(0..0, offsets.rev());
            self.displayed_lines.truncate(height - filler);
        } else {
            // Otherwise, append in order
            if reversed {
                self.displayed_lines.extend(offsets.rev());
            } else {
                self.displayed_lines.extend(offsets);
            }
            let skip = self.displayed_lines.len().saturating_sub(height - filler);
            if skip > 0 {
                self.displayed_lines = self.displayed_lines[skip..].to_vec();
            }
        }

        Ok(buff)
    }

    fn get_max_pan(&self, doc: &mut Document, scroll: &Scroll) -> usize {
        let sv = match scroll {
            Scroll::Repaint(ref sv) => sv,

            Scroll::Up(_) |
            Scroll::Down(_) |
            Scroll::GotoBottom(_) |
            Scroll::Overwrite(..) |
            Scroll::GotoTop(_) => panic!("Expected Repaint after PanRightMax"),

            Scroll::None => unreachable!("Scroll::None")
        };

        let max =
            doc.get_plain_lines(&(sv.offset..))
                .map(|line| line.line.len())
                .take(sv.lines)
                .max();
        if let Some(max) = max {
            // FIXME: +1 because of \n on EOL; but we might strip it in the future
            max.saturating_sub(self.width + 1)
        } else {
            0
        }
    }

    // Pull lines from an iterator and display them.  There are three modes:
    // 1. Scroll up:  Display each new line at the next lower position, and scroll up from bottom
    // 2. Scroll down:  Display each new line at the next higher position, and scroll down from top
    // 3. Repaint:  Display all lines from the given offset
    // pos is the offset in the file for the first line
    // Scroll distance is in screen rows.  If a read line takes multiple rows, they count as multiple lines.
    fn feed_lines(&mut self, doc: &mut Document, scroll: Scroll) -> std::io::Result<ScreenBuffer> {
        log::trace!("feed_lines: {:?}", scroll);

        let height = self.page_size();

        if self.pan == usize::MAX {
            self.pan = self.get_max_pan(doc, &scroll);
        }

        if self.chop && self.pan == 0 {
            doc.set_line_mode(LineViewMode::Wrap{width: self.width});
        } else {
            // Pan the document to the left; override wrap-mode
            doc.set_line_mode(LineViewMode::Clip{width: self.width, left: self.pan});
        }

        let lines= match scroll {
            Scroll::Up(ref sv) | Scroll::GotoBottom(ref sv) => {
                // Partial or complete screen scroll backwards
                let skip = sv.lines.saturating_sub(height);
                let range = ..sv.offset;
                let lines:Vec<_> = doc.get_lines_range(&range).rev().take(sv.lines).skip(skip).collect();
                lines
            },
            Scroll::Down(ref sv) => {
                // Partial screen scroll forwards
                let skip = sv.lines.saturating_sub(height);
                let range = sv.offset..;
                let mut lines = doc.get_lines_range(&range).take(sv.lines + 1);
                if let Some(line) = lines.next() {
                    // When scrolling down, the first line retrieved is the bottom line from the previous screen
                    // because that's where we start.  We don't know the offset of the next line, so we always get the
                    // bottom line again, redundantly.
                    assert_eq!(line.offset, sv.offset);
                }
                lines.skip(skip).collect()
            },
            Scroll::Repaint(ref sv) | Scroll::GotoTop(ref sv) => {
                // Repainting whole screen, no scrolling
                let skip = sv.lines.saturating_sub(height);
                let range = sv.offset..;
                let lines = doc.get_lines_range(&range).take(sv.lines.min(height));
                lines.skip(skip).collect()
            },
            Scroll::Overwrite(_, ref sv) => {
                // Overwrite a range of lines with new data
                let skip = sv.lines.saturating_sub(height);
                let range = sv.offset..;
                let lines = doc.get_lines_range(&range).skip_while(|line| line.offset <= sv.offset).take(sv.lines.min(height));
                lines.skip(skip).collect()
            },
            Scroll::None => unreachable!("Scroll::None")
        };

        self.paint(doc, lines, scroll)
    }

    fn have_new_lines(&self, doc: &mut Document) -> bool {
        if self.displayed_lines.len() < self.page_size() {
            let len = doc.len();
            let new_len = doc.poll(None);
            new_len > len
        } else {
            false
        }
    }

    pub fn refresh_screen(&mut self, doc: &mut Document) -> std::io::Result<()> {
        // FIXME: Discard unused cached lines

        let view_height = self.page_size();

        // Our new display
        let disp = DisplayState {
            height: self.page_size(),
            width: self.width,
            pan: self.pan,
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
            } else if self.have_new_lines(doc) {
                // We displayed fewer lines than the screen height. Check for more data appearing
                log::trace!("check for more data");
                Scroll::overwrite(*self.displayed_lines.last().unwrap(), self.displayed_lines.len(), view_height)
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
                        log::trace!("scroll to top: line {line}");
                        Scroll::goto_top(0, view_height + line.saturating_sub(1))
                    }
                    ScrollAction::EndOfFile(line) => {
                        // Scroll to bottom
                        log::trace!("scroll to bottom: line -{line}");
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
                    ScrollAction::Search(forward, repeat) => {
                        let begin = if !forward {
                            // Search backwards from the first line displayed
                            log::trace!("search backward");
                            doc.search_back(*self.displayed_lines.first().unwrap(), repeat)
                        } else {
                            // Search forwards from the last line displayed
                            log::trace!("search forward");
                            doc.search_next(*self.displayed_lines.last().unwrap(), repeat)
                        };
                        if let Some(begin) = begin {
                            Scroll::repaint(begin, view_height)
                        } else {
                            Scroll::repaint(*self.displayed_lines.first().unwrap(), view_height)
                        }
                    }
                    ScrollAction::None => Scroll::none()
                }
            };

        self.scroll = ScrollAction::None;

        if plan.is_none() {
            return Ok(());
        }

        log::trace!("screen changed");

        let mut buff = self.feed_lines(doc, plan)?;
        self.prev = disp;

        // DEBUG HACK
        // self.draw_line(doc, &mut buff, self.height - 2, &format!("scroll={} displayed={:?}", scroll, self.displayed_lines));
        buff.flush()
    }

}