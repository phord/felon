use crossterm::style::Color;
use crossterm::{terminal::ClearType};
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, execute, queue, terminal};
use crate::config::Config;
use crate::keyboard::UserCommand;
use crate::styled_text::{PattColor, RegionColor, StyledLine};
use crate::document::Document;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Action {
    Idle,
    Progress {percent: usize, },
    SearchPrompt,
    Message,
}

#[derive(PartialEq)]
struct DisplayState {
    top: usize,
    bottom: usize,
    // offset: usize, // column offset
    width: usize,
    status: Action,
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

pub struct Display {
    // Physical size of the display
    height: usize,
    width: usize,
    on_alt_screen: bool,
    use_alt: bool,


    /// First line on the display (line-number in the file)
    top: usize,

    /// Size of the bottom status panel
    panel: usize,

    /// Previously displayed lines
    prev: DisplayState,

    /// Activity mode
    action: Action,

    /// status message
    message: String,

    mouse_wheel_height: u16,

}

impl Drop for Display {
    fn drop(&mut self) {
        if self.on_alt_screen {
            execute!(stdout(), terminal::LeaveAlternateScreen).expect("Failed to exit alt mode");
        }
    }
}

impl Display {
    pub fn new(config: Config) -> Self {
        let mut s = Self {
            height: 0,
            width: 0,
            on_alt_screen: false,
            use_alt: config.altscreen,
            top: 0,
            panel: 1,
            prev: DisplayState { top: 0, bottom: 0, width: 0, status: Action::Idle, },
            action: Action::Message,
            message: "status message".to_string(),
            mouse_wheel_height: config.mouse_scroll,
        };
        s.update_size();
        s
    }

    pub fn search_prompt(&mut self) {
        self.action = Action::SearchPrompt;
    }

    fn update_size(&mut self) {
        let (width, height) = terminal::size().expect("Unable to get terminal size");
        self.width = width as usize;
        self.height = height as usize;
    }

    fn page_size(&self) -> usize {
        cmp::max(self.height as isize - self.panel as isize, 0) as usize
    }

    fn status_msg(&mut  self) -> &str {
        self.action = Action::Idle;
        self.message.as_str()
    }

    fn set_status_msg(&mut self, msg: String) {
        self.message = msg;
        self.action = Action::Message;
    }

    fn vert_scroll(&mut self, amount: isize) {
        let top = self.top as isize + amount;
        self.top = cmp::max(top, 0) as usize;
    }


    pub fn handle_command(&mut self, cmd: UserCommand) {
        match cmd {
            UserCommand::ScrollDown => {
                self.vert_scroll(1);
            }
            UserCommand::ScrollUp => {
                self.vert_scroll(-1);
            }
            UserCommand::PageDown => {
                self.vert_scroll(self.page_size() as isize);
            }
            UserCommand::PageUp => {
                self.vert_scroll(-(self.page_size() as isize));
            }
            UserCommand::ScrollToTop => {
                self.top = 0;
            }
            UserCommand::ScrollToBottom => {
                self.top = usize::MAX;
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
                self.vert_scroll(-(self.mouse_wheel_height as isize));
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::MouseScrollDown => {
                self.vert_scroll(self.mouse_wheel_height as isize);
                self.set_status_msg(format!("{:?}", cmd));
            }
            _ => {}
        }
    }

    fn status_line_colors(&self, line: &str) -> StyledLine {
        StyledLine::new(line, PattColor::Inverse)
    }

    fn draw_styled_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: StyledLine) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.set_width(self.width);
        buff.push(line);

        queue!(buff, crossterm::style::SetBackgroundColor(Color::Black), terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_line(&mut self, doc: &mut Document, buff: &mut ScreenBuffer, row: usize, line: &String) {
        // TODO: Memoize the line_colors along with the lines
        self.draw_styled_line(buff, row, doc.line_colors(line));
    }

    fn draw_status_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: &String) {
        self.draw_styled_line(buff, row, self.status_line_colors(line));
    }

    pub fn refresh_screen(&mut self, doc: &mut Document) -> crossterm::Result<()> {
        // FIXME: Discard unused cached lines

        let view_height = self.page_size();
        self.top = cmp::min(self.top, doc.lines().saturating_sub(view_height));

        if ! self.on_alt_screen && self.use_alt {
            execute!(stdout(), terminal::EnterAlternateScreen)?;
            self.on_alt_screen = true;
        }

        // What we want to display
        let disp = DisplayState {
            top: self.top,
            bottom: self.top + self.page_size(),
            width: self.width,
            status: self.action,
        };

        if disp == self.prev {
            // No change; nothing to do.
            return Ok(());
        }

        // Calc screen difference from previous display in scroll-lines (pos or neg)
        let scroll = disp.top as isize - self.prev.top as isize;
        let (scroll, top, bottom) =
            if scroll == 0 {
                // No scrolling; check height/width
                if disp.width <= self.prev.width {
                    if self.page_size() <= self.prev.bottom - self.prev.top {
                        // Screen is the same or smaller. Nothing to do.
                        (0, disp.bottom, disp.bottom)
                    } else {
                        // Just need to display new rows at bottom
                        (0, self.prev.bottom, disp.bottom)
                    }
                } else {
                    // Screen got wider.  Repaint everything.
                    (0, disp.top, disp.bottom)
                }
            } else if scroll.abs() > self.page_size() as isize {
                // Scrolling too far; clear the screen
                (0, disp.top, disp.bottom)
            } else if scroll < 0 {
                // Scroll down
                (scroll, (disp.top as isize) as usize, self.prev.top)
            } else if scroll > 0 {
                // Scroll up
                (scroll, self.prev.bottom, self.prev.bottom + scroll as usize)
            } else {
                unreachable!("Unexpected scroll value: {}", scroll);
            };

        assert!(top >= disp.top);

        let len = bottom - top;
        let start = top - disp.top;

        self.prev = disp;

        let mut buff = ScreenBuffer::new();

        if scroll < 0 {
            queue!(buff, terminal::ScrollDown(scroll.abs() as u16)).unwrap();
        } else if scroll > 0 {
            queue!(buff, terminal::ScrollUp(scroll as u16)).unwrap();
        } else {
            // Clear the screen? Unnecessary.
        }
        queue!(buff, cursor::Hide)?;

        for row in start..start+len as usize {
            let lrow = self.top + row;
            let line = doc.get(lrow).to_string();
            self.draw_line(doc, &mut buff, row, &line);
        }

        if self.panel > 0 {
            for row in self.height-self.panel..self.height as usize {
                match self.action {
                    Action::Idle => {
                        let line = &self.status_msg().to_string();
                        self.draw_status_line(&mut buff, row, line);
                    },
                    Action::Message => {
                        let line = &self.status_msg().to_string();
                        self.draw_status_line(&mut buff, row, line);
                    }
                    Action::SearchPrompt => {
                        self.draw_status_line(&mut buff, row, &"/".to_string());
                    }
                    _ => {
                        self.draw_status_line(&mut buff, row, &"Twiddling thumbs".to_string());
                }
                };
            }
        }

       buff.flush()
    }

}