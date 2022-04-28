use crossterm::{terminal::ClearType, cursor::DisableBlinking};
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, event, execute, queue, terminal};
use crate::config::Config;
use crate::keyboard::UserCommand;
use std::collections::HashMap;

#[derive(PartialEq)]
struct DisplayState {
    top: usize,
    bottom: usize,
    // offset: usize, // column offset
    width: usize,
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
    bottom: usize,

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
        let (width, height) = terminal::size().expect("Unable to get terminal size");
        Self {
            height: height as usize,
            width: width as usize,
            data: HashMap::new(),
            on_alt_screen: false,
            use_alt: config.altscreen,
            top: 0,
            bottom: height as usize,
            prev: DisplayState { top: 0, bottom: 0, width: 0 },
        }
    }

    pub fn push(&mut self, row: usize, line: &str) {
        self.data.insert(row, line.to_string());
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn set_length(&mut self, length: usize) {
        self.bottom = length;
    }

    pub fn lines_needed(&self) -> Vec<usize> {
        let lines = (self.top..self.top + self.height)
            .filter(|x| {!self.data.contains_key(x)} )
            .collect();
        lines
    }

    fn vert_scroll(&mut self, amount: isize) {
        let top = self.top as isize + amount;
        let top = cmp::max(top, 0) as usize;

        self.top = if top + self.height >= self.bottom {
            self.bottom.saturating_sub(self.height)
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
                self.vert_scroll(self.height as isize);
            }
            UserCommand::PageUp => {
                self.vert_scroll(-(self.height as isize));
            }
            UserCommand::ScrollToTop => {
                self.top = 0;
            }
            UserCommand::ScrollToBottom => {
                self.vert_scroll(self.bottom as isize);
            }
            UserCommand::TerminalResize => {
                let (width, height) = terminal::size().expect("Unable to get terminal size");
                self.width = width as usize;
                self.height = height as usize;
            }
            _ => {}
        }
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
            bottom: self.top + self.height,
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
                    if self.height <= self.prev.bottom - self.prev.top {
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
            } else if scroll.abs() > self.height as isize {
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
        queue!(buff, cursor::Hide, cursor::MoveTo(0, start as u16))?;

        if scroll < 0 {
            queue!(buff, terminal::ScrollDown(scroll.abs() as u16)).unwrap();
        } else if scroll > 0 {
            queue!(buff, terminal::ScrollUp(scroll as u16)).unwrap();
        } else {
            // Clear the screen? Unnecessary.
        }

        for row in start..start+len as usize {
            let lrow = self.top + row;
            let line = self.data.get(&lrow);
            match line {
                Some(line) => {
                    let len = cmp::min(line.len(), self.width as usize);
                    buff.push_str(&line[0..len]);
                }
                _ => {
                    buff.push('~');
                }
            }

            queue!(buff, terminal::Clear(ClearType::UntilNewLine)).unwrap();

            if row < self.height as usize - 1 {
                buff.push_str("\r\n");
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