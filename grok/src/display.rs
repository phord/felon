use crossterm::terminal::ClearType;
use std::io::stdout;
use crossterm::{cursor, event, execute, queue, terminal};
use std::io::Write;
use crate::config::Config;


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
    pub height: u16,
    width: u16,
    data: Vec<String>,
    on_alt_screen: bool,
    use_alt: bool,
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
            height,
            width,
            data: Vec::new(),
            on_alt_screen: false,
            use_alt: config.altscreen,
        }
    }

    pub fn push(&mut self, line: &str) {
        self.data.push(line.to_string());
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn refresh_screen(&mut self) -> crossterm::Result<()> {
        if ! self.on_alt_screen && self.use_alt {
            execute!(stdout(), terminal::EnterAlternateScreen)?;
            self.on_alt_screen = true;
        }

        let mut buff = ScreenBuffer::new();
        queue!(buff, cursor::Hide, cursor::MoveTo(0, 0))?;
        // self.draw_rows();

        for row in 0..self.height as usize {
            // buff.push_str(&welcome);
            if row >= self.data.len() {
                buff.push('~');
            } else {
                let line = self.data[row].clone();  // FIXME: Avoid clone by using lifetime?
                let len = std::cmp::min(line.len(), self.width as usize);
                buff.push_str(&line[0..len]);
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