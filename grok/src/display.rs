use crossterm::terminal::ClearType;
use std::io::stdout;
use crossterm::{cursor, event, execute, queue, terminal};
use std::io::Write;
use crate::config::Config;
use crate::keyboard::UserCommand;
use std::collections::HashMap;

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

use std::io;

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
    pub height: usize,
    width: usize,
    data: HashMap<usize, String>,
    on_alt_screen: bool,
    use_alt: bool,

    top: usize,
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
        }
    }

    pub fn push(&mut self, row: usize, line: &str) {
        self.data.insert(row, line.to_string());
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn set_length(&mut self, length: usize) {
        // TODO:
        // self.last_line = length;
    }

    pub fn lines_needed(&self) -> Vec<usize> {
        let mut lines = Vec::new();
        lines = (self.top..self.top+self.height).collect();
        lines
    }

    pub fn handle_command(&mut self, cmd: UserCommand) {
        match cmd {
            UserCommand::ScrollDown => {
                self.top += 1;
            }
            UserCommand::ScrollUp => {
                if self.top > 0 {
                    self.top -= 1;
                }
            }
            UserCommand::PageDown => {
                self.top += self.height;
            }
            UserCommand::PageUp => {
                if self.top > self.height {
                    self.top -= self.height;
                } else {
                    self.top = 0;
                }
            }
            _ => {}
        }
    }

    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        // FIXME: Don't do anything if the screen has not changed
        // FIXME: Only update parts of the screen that have changed
        // FIXME: Discard unused cached lines

        if ! self.on_alt_screen && self.use_alt {
            execute!(stdout(), terminal::EnterAlternateScreen)?;
            self.on_alt_screen = true;
        }

        let mut buff = ScreenBuffer::new();
        queue!(buff, cursor::Hide, cursor::MoveTo(0, 0))?;

        for row in 0..self.height as usize {
            let lrow = self.top + row;
            let line = self.data.get(&lrow);
            match line {
                Some(line) => {
                    let len = std::cmp::min(line.len(), self.width as usize);
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