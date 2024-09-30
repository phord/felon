use crossterm::terminal::ClearType;
use indexed_file::LineViewMode;
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, execute, queue, terminal};
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
    StartOfFile,
    EndOfFile,
    Up(usize),
    Down(usize),
}

pub struct Display {
    // Physical size of the display
    height: usize,
    width: usize,
    on_alt_screen: bool,
    use_alt: bool,

    /// Scroll command from user
    scroll: ScrollAction,

    /// Size of the bottom status panel
    panel: usize,

    /// Previous display info
    prev: DisplayState,

    // Displayed line offsets
    displayed_lines: Vec<usize>,

    mouse_wheel_height: u16,

    mode: LineViewMode,

}

impl Drop for Display {
    fn drop(&mut self) {
        log::trace!("Display closing");
        self.stop().expect("Failed to stop display");
    }
}

impl Display {
    pub fn new(config: Config) -> Self {
        let s = Self {
            height: 0,
            width: 0,
            on_alt_screen: false,
            use_alt: config.altscreen,
            scroll: ScrollAction::StartOfFile,
            panel: 1,
            prev: DisplayState { height: 0, width: 0},
            displayed_lines: Vec::new(),
            mouse_wheel_height: config.mouse_scroll,
            mode: LineViewMode::WholeLine,
        };
        s
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

    pub fn handle_command(&mut self, cmd: UserCommand) {
        match cmd {
            UserCommand::ScrollDown => {
                self.scroll = ScrollAction::Down(1);
            }
            UserCommand::ScrollUp => {
                self.scroll = ScrollAction::Up(1);
            }
            UserCommand::PageDown => {
                self.scroll = ScrollAction::Down(self.page_size());
            }
            UserCommand::PageUp => {
                self.scroll = ScrollAction::Up(self.page_size());
            }
            UserCommand::ScrollToTop => {
                self.scroll = ScrollAction::StartOfFile;
            }
            UserCommand::ScrollToBottom => {
                self.scroll = ScrollAction::EndOfFile;
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
            _ => {}
        }
    }

    fn draw_styled_line(&self, buff: &mut ScreenBuffer, row: usize, line: StyledLine) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.set_width(self.width);
        buff.push(line);

        queue!(buff, crossterm::style::SetBackgroundColor(RGB_BLACK), terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_line(&self, doc: &Document, buff: &mut ScreenBuffer, row: usize, line: &String) {
        // TODO: Memoize the line_colors along with the lines
        self.draw_styled_line(buff, row, doc.line_colors(line));
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
    fn none() -> Self {
        Self::None
    }
    fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }
}

impl Display {
    // Pull lines from an iterator and display them.  There are two modes:
    // 1. Scroll up:  Display each new line at the next lower position, and scroll up from bottom
    // 2. Scroll down:  Display each new line at the next higher position, and scroll down from top
    // pos is the offset in the file for the first line
    // if size is larger than display height, we may skip unnecessary lines
    // Scroll distance is in screen rows.  If a read line takes multiple rows, they count as multiple lines.
    fn feed_lines(&mut self, doc: &mut Document, mode: LineViewMode, scroll: Scroll) -> crossterm::Result<ScreenBuffer> {
        log::trace!("feed_lines: {:?}", scroll);

        let mut buff = ScreenBuffer::new();

        let top_of_screen = 0;

        // FIXME: Handle case when lines are shorter than display area

        let (lines, mut row, mut count) = match scroll {
            Scroll::Up(sv) => {
                // Partial or complete screen scroll backwards
                let lines: Vec<_> = doc.get_lines_from_rev(mode, sv.offset, sv.lines).into_iter().rev().collect();
                let rows = lines.len();
                queue!(buff, terminal::ScrollDown(rows as u16)).unwrap();
                self.displayed_lines.splice(0..0, lines.iter().map(|(pos, _)| *pos).take(rows));
                self.displayed_lines.truncate(self.page_size());
                // TODO: add test for whole-screen offsets == self.displayed_lines
                (lines, 0, rows)
            },
            Scroll::Down(sv) => {
                // Partial screen scroll forwards
                let mut lines = doc.get_lines_from(mode, sv.offset, sv.lines + 1);
                if !lines.is_empty() {
                    let skipped = lines.remove(0);
                    assert_eq!(skipped.0, sv.offset);
                }
                let rows = lines.len();
                queue!(buff, terminal::ScrollUp(rows as u16)).unwrap();
                self.displayed_lines = if self.displayed_lines.len() > rows {
                    self.displayed_lines[rows as usize..].to_vec()
                } else {
                    Vec::new()
                };
                self.displayed_lines.extend(lines.iter().map(|(pos, _)| *pos).take(rows));
                (lines, self.page_size() - rows, rows)
            },
            Scroll::Repaint(sv) => {
                // Repainting whole screen, no scrolling
                // FIXME: Clear to EOL after each line instead of clearing screen
                let lines = doc.get_lines_from(mode, sv.offset, sv.lines);
                let rows = lines.len();
                queue!(buff, terminal::Clear(ClearType::All)).unwrap();
                self.displayed_lines = lines.iter().map(|(pos, _)| *pos).take(rows).collect();
                (lines, 0, rows)
            },
            Scroll::None => unreachable!("Scroll::None")
        };

        for (pos, line) in lines.iter(){
            assert_eq!(self.displayed_lines[row - top_of_screen],  *pos);
            self.draw_line(doc, &mut buff, row, line);
            row += 1;
            count -= 1;
        }

        while count > 0 {
            self.draw_line(doc, &mut buff, row, &"~".to_string());
            row += 1;
            count -= 1;
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
            if self.displayed_lines.len() == 0 {
                // Blank slate; start of file
                log::trace!("start of file");
                Scroll::repaint(0, view_height)
            } else if disp != self.prev {
                // New screen dimensions; repaint everything
                // FIXME: No need to repaint if we got smaller
                // FIXME: Only need to add rows if we only got taller
                log::trace!("repaint everything");
                Scroll::repaint(*self.displayed_lines.first().unwrap(), view_height)
            } else {
                match self.scroll {
                    ScrollAction::StartOfFile => {
                        // Scroll to top
                        log::trace!("scroll to top");
                        // Scroll::repaint(0, 0, view_height)
                        Scroll::repaint(0, view_height)
                    }
                    ScrollAction::EndOfFile => {
                        // Scroll to bottom
                        log::trace!("scroll to bottom");
                        // FIXME: iterator isn't returning rows from the end.  Why?
                        Scroll::up(usize::MAX/2, view_height)
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