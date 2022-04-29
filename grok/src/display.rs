use crossterm::{terminal::ClearType, style::Stylize, style::ContentStyle};
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, execute, queue, terminal};
use crate::config::Config;
use crate::keyboard::UserCommand;
use std::collections::HashMap;

use crossterm::style::Color;

#[derive(PartialEq)]
struct DisplayState {
    top: usize,
    bottom: usize,
    // offset: usize, // column offset
    width: usize,
}

enum PattColor {
    Normal,
    Highlight,
    Inverse,
}
/// Line section coloring
struct RegionColor {
    len: u16,
    style: PattColor,
}

impl RegionColor {
    fn to_str(&self, line: &str) -> String {
        let len = cmp::min(self.len as usize, line.len());
        let content = &line[..len];
        let style = ContentStyle::new();

        let style = match self.style {
            PattColor::Normal => style.reset(),
            PattColor::Highlight => style.with(Color::Red),
            PattColor::Inverse => style.negative(),
        };
        format!("{}" , style.apply(content))
    }
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

    fn push(&mut self, ch: char) {
        self.content.push(ch)
    }

    fn push_str(&mut self, string: &str) {
        self.content.push_str(string)
    }
}

impl io::Write for ScreenBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.content.push_str(s);
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
            prev: DisplayState { top: 0, bottom: 0, width: 0 },
        };
        s.update_size();
        s
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

    fn status_msg(& self) -> String {
        "Status message".to_string()
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
            _ => {}
        }
    }

    fn line_colors(&self, line: &str) -> Vec<RegionColor> {
        let len = line.len();
        vec![
            RegionColor { len: 20, style: PattColor::Normal },
            RegionColor { len: 10, style: PattColor::Highlight },
            RegionColor { len: len as u16, style: PattColor::Normal },
        ]
    }

    fn status_line_colors(&self, line: &str) -> Vec<RegionColor> {
        let len = line.len();
        vec![
            RegionColor { len: len as u16, style: PattColor::Inverse },
        ]
    }

    fn draw_styled_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: &String, colors: &Vec<RegionColor>) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        let len = cmp::min(line.len(), self.width as usize);
        let mut pos = 0;
        for c in colors {
            let section = cmp::min(c.len as usize, len - pos);
            let str = c.to_str(&line[pos..pos+section]);
            buff.push_str(&format!("{}", str));
            pos += section;
            if pos == len { break; }
        }

        queue!(buff, terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: &String) {
        self.draw_styled_line(buff, row, line, &self.line_colors(line));
    }

    fn draw_status_line(&mut self, buff: &mut ScreenBuffer, row: usize, line: &String) {
        self.draw_styled_line(buff, row, line, &self.status_line_colors(line));
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
            width: self.width
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
                        (0, 0, 0)
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


        if top == bottom {
            // Nothing to do
            self.prev = disp;
            return Ok(());
        }

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
                self.draw_status_line(&mut buff, row, &self.status_msg());
            }
        }

        queue!(
            buff,
            cursor::MoveTo(0, 0),
            cursor::Show
        )?;
        buff.flush()
    }

}