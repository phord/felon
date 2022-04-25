use crossterm::event::{Event, KeyCode, KeyEvent};
use crossterm::{cursor, event, execute, queue, terminal};
use crossterm::terminal::ClearType;
use std::io::stdout;
use std::cmp;
use std::time::Duration;
use std::io::Write;


pub enum UserCommand {
    None,
    Quit,
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
}

struct Reader;
impl Reader {
    fn read_key(&self) -> crossterm::Result<KeyEvent> {
        loop {
            if event::poll(Duration::from_millis(500))? {
                if let Event::Key(event) = event::read()? {
                    return Ok(event);
                }
            }
        }
    }
}

pub struct Input {
    reader: Reader,
    started: bool,
}

impl Drop for Input {
    fn drop(&mut self) {
        if self.started {
            terminal::disable_raw_mode().expect("Unable to disable raw mode");
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            reader: Reader,
            started: false,
        }
    }

    fn start(&mut self) -> crossterm::Result<()> {
        if !self.started {
            terminal::enable_raw_mode()?;
            self.started = true;
        }
        Ok(())
    }

    pub fn read_key(&mut self) -> crossterm::Result<KeyEvent> {
        self.start()?;

        // TODO: Map keymap -> opcodes; eg. ScrollDown, ScrollUp, ScrollPageDown, etc.
        // TODO: Different keymaps for different modes. user-input, scrolling, etc.
        self.reader.read_key()
    }

    pub fn process_keypress(&mut self) -> crossterm::Result<UserCommand> {
        match self.read_key()? {
            KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: event::KeyModifiers::CONTROL,
            } => return Ok(UserCommand::Quit),
            KeyEvent {
                code: KeyCode::Esc,
                modifiers: event::KeyModifiers::NONE,
            } => return Ok(UserCommand::Quit),
            // KeyEvent {
            //     code: direction @ (KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
            //                        | KeyCode::Home | KeyCode::End),
            //     modifiers: event::KeyModifiers::NONE,
            // } => self.output.move_cursor(direction),
            // KeyEvent {
            //     code: val @ (KeyCode::PageUp | KeyCode::PageDown),
            //     modifiers: event::KeyModifiers::NONE,
            // } => (0..self.output.win_size.1).for_each(|_| {
            //     self.output.move_cursor(if matches!(val, KeyCode::PageUp) {
            //         KeyCode::Up
            //     } else {
            //         KeyCode::Down
            //     });
            // }),
            _ => {}
        }
    Ok(UserCommand::None)
    }
}
