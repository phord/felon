use crossterm::terminal::ClearType;
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, event, execute, queue, terminal};
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
    bottom: usize,
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