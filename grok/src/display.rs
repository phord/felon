use crossterm::style::Color;
use crossterm::{terminal::ClearType};
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, execute, queue, terminal};
use crate::config::Config;
use crate::keyboard::UserCommand;
use std::collections::HashMap;
use lazy_static::lazy_static;
use regex::Regex;

use fnv::FnvHasher;
use std::hash::Hasher;

use crate::styled_text::{PattColor, RegionColor, StyledLine};

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
    data: HashMap<usize, String>,
    on_alt_screen: bool,
    use_alt: bool,


    /// First line on the display (line-number in the file)
    top: usize,

    /// Size of the bottom status panel
    panel: usize,

    /// Total lines in the file
    lines_count: usize,

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
            data: HashMap::new(),
            on_alt_screen: false,
            use_alt: config.altscreen,
            top: 0,
            panel: 1,
            lines_count: 0,
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

    pub fn push(&mut self, row: usize, line: &str) {
        self.data.insert(row, line.to_string());
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn set_length(&mut self, length: usize) {
        self.lines_count = length;
    }

    pub fn lines_needed(&self) -> Vec<usize> {
        let lines = (self.top..self.top + self.page_size())
            .filter(|x| {!self.data.contains_key(x)} )
            .collect();
        lines
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
        let top = cmp::max(top, 0) as usize;

        let view_height = self.page_size();
        self.top = if top + view_height >= self.lines_count {
            self.lines_count.saturating_sub(view_height)
        } else { top };

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
                self.vert_scroll(self.lines_count as isize);
            }
            UserCommand::TerminalResize => {
                self.update_size();
            }
            UserCommand::SelectWordAt(x,y) => {
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::SelectWordDrag(x,y) => {
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

    fn hash_color(&self, text: &str) -> Color {
        let mut hasher = FnvHasher::default();
        hasher.write(text.as_bytes());
        let hash = hasher.finish();

        let base = 0x80 as u8;
        let red = (hash & 0xFF) as u8 | base;
        let green = ((hash >> 8) & 0xFF) as u8 | base;
        let blue = ((hash >> 16) & 0xFF) as u8 | base;

        Color::Rgb {r: red, g: green, b: blue}
    }

    // TODO: Move this to another module. "context.rs"?
    fn line_colors(&self, line: &str) -> StyledLine {
        lazy_static! {
            // Apr  4 22:21:16.056 E8ABF4F03A6F I      vol.flush.cb ...
            static ref TIMESTAMP: Regex = Regex::new(r"(?x)
                ^(...\ [\ 1-3]\d\ [0-2]\d:[0-5]\d:\d{2}\.\d{3})\    # date & time
                 ([A-F0-9]{12})\                                    # PID
                 ([A-Z])\                                           # crumb").unwrap();

            static ref MODULE: Regex = Regex::new(r"(?x)
                 ^\ *([A-Za-z0-9_.]+)\                              # module
                 (?:\[([a-z0-9_.]+)\]){0,1}                         # submodule").unwrap();

            static ref NUMBER: Regex = Regex::new(r"[^A-Za-z_.](0x[[:xdigit:]]+|(?:[[:digit:]]+\.)*[[:digit:]]+)").unwrap();
        }
        let prefix = TIMESTAMP.captures(line);

        let mut styled = StyledLine::new(line, PattColor::NoCrumb);

        // Match and color PID and TIME
        let mut pos = 0;
        if let Some(p) = prefix {
            let crumb = p.get(3).unwrap().as_str();
            let default_style = match crumb.as_ref() {
                "E" => PattColor::Error,
                "A" => PattColor::Fail,
                _ => PattColor::Info,
            };

            styled.push(0, line.len(), default_style);

            let len = p.get(1).unwrap().end() + 1;
            styled.push(0, len, PattColor::Timestamp);

            // TODO: Calculate timestamp value?

            let pid = p.get(2).unwrap();
            let start = pid.start();
            let end = pid.end();
            let pid = pid.as_str();
            let pid_color = self.hash_color(pid);
            styled.push( start, end, PattColor::Pid(pid_color));

            // Match modules at start of line
            pos = end + 3;  // Skip over crumb; it will autocolor later
            let module = MODULE.captures(&line[pos..]);
            if let Some(m) = module {
                let first = m.get(1).unwrap();
                let color = self.hash_color(first.as_str());
                styled.push(pos + first.start(), pos + first.end(),PattColor::Module(color) );

                if let Some(second) = m.get(2) {
                    let color = self.hash_color(second.as_str());
                    styled.push(pos + second.start(), pos + second.end(), PattColor::Module(color));
                }
            }
        }

        for m in NUMBER.captures_iter(&line[pos..]) {
            let m = m.get(1).unwrap();
            let start = m.start();
            let end = m.end();
            let color = self.hash_color(m.as_str());
            styled.push( pos + start, pos + end , PattColor::Number(color) );
        }

        styled
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

    fn draw_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: &String) {
        // TODO: Memoize the line_colors along with the lines
        self.draw_styled_line(buff, row, self.line_colors(line));
    }

    fn draw_status_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: &String) {
        self.draw_styled_line(buff, row, self.status_line_colors(line));
    }

    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        // FIXME: Discard unused cached lines

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
            let line = self.data.get(&lrow);
            let line = line.unwrap_or(&'~'.to_string()).clone();
            self.draw_line(&mut buff, row, &line);
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