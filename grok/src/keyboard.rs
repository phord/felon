use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{event, terminal, execute};
use std::collections::HashMap;
use std::time::Duration;
use std::io::stdout;

use UserCommand as cmd;
const KEYMAP: &'static [(&'static str, UserCommand)] = &[
    ("Ctrl+W", cmd::Quit),
    ("Q", cmd::Quit),
    ("Esc", cmd::Quit),
    ("Up", cmd::ScrollUp),
    ("Down", cmd::ScrollDown),
    ("PageUp", cmd::PageUp),
    ("PageDown", cmd::PageDown),
    ("Home", cmd::ScrollToTop),
    ("End", cmd::ScrollToBottom),
];

#[derive(Copy, Clone)]
pub enum UserCommand {
    None,
    Quit,
    Init,       // first screen
    ScrollUp,
    ScrollDown,
    PageUp,
    PageDown,
    ScrollToTop,
    ScrollToBottom,
    TerminalResize,
}

struct Reader {
    keymap: HashMap<KeyEvent, UserCommand>,
}

impl Reader {

    pub fn new() -> Self {
        let keymap: HashMap<_, _> = KEYMAP
            .iter()
            .map(|(key, cmd)| (Self::keycode(key).unwrap(), *cmd))
            .collect();

        Self {
            keymap,
        }
    }

    /// Convert a string representation of a key combo into a KeyEvent
    /// ```
    /// assert_eq!(crate::Input::keycode("Ctrl+Q"), KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    /// ```
    fn keycode(orig: &str) -> Result<KeyEvent, String> {
        let mut result = KeyEvent::new(KeyCode::Null, KeyModifiers::NONE);

        let str = orig.to_lowercase();
        for key in str.split("+") {
            let mods = match key {
                "shift" => crossterm::event::KeyModifiers::SHIFT,
                "alt" => crossterm::event::KeyModifiers::ALT,
                "ctrl" => crossterm::event::KeyModifiers::CONTROL,
                _ => crossterm::event::KeyModifiers::NONE,
            };

            let action = match key {
                "backspace" => KeyCode::Backspace,
                "enter" => KeyCode::Enter,
                "left" => KeyCode::Left,
                "right" => KeyCode::Right,
                "up" => KeyCode::Up,
                "down" => KeyCode::Down,
                "home" => KeyCode::Home,
                "end" => KeyCode::End,
                "pageup" => KeyCode::PageUp,
                "pagedown" => KeyCode::PageDown,
                "tab" => KeyCode::Tab,
                "backtab" => KeyCode::BackTab,
                "delete" => KeyCode::Delete,
                "insert" => KeyCode::Insert,
                "null" => KeyCode::Null,
                "esc" => KeyCode::Esc,
                k => {
                    if k.len() == 1 {
                        KeyCode::Char(k.chars().next().unwrap())
                    } else if k.len() > 1 && k.starts_with("F") && k.len() < 4 {
                        KeyCode::F(k[1..].parse().unwrap())
                    } else {
                        KeyCode::Null
                    }
                }
            };

            if mods != KeyModifiers::NONE {
                if result.modifiers & mods != KeyModifiers::NONE {
                    return Err(format!("Key combo {} gives {} twice", orig, key));
                }
                result.modifiers |= mods;
            } else if action != KeyCode::Null {
                // Already got an action key
                if result.code != KeyCode::Null {
                    return Err(format!("Key combo {} has two action keys", orig));
                }
                result.code = action;
            } else {
                return Err(format!("Unknown key name {} in {}", key, orig));
            }
        }
        Ok(result)
    }

    fn get_command(&self) -> crossterm::Result<UserCommand> {
        loop {
            if event::poll(Duration::from_millis(500))? {

                match event::read()? {
                    Event::Key(event) => {
                        return match self.keymap.get(&event) {
                            Some(cmd) => Ok(*cmd),
                            None => Ok(UserCommand::None),
                        };
                    }
                    Event::Mouse(event) => {
                        println!("{:?}", event);
                        // FIXME: handle mouse events
                        return Ok(cmd::None);
                    }
                    Event::Resize(width, height) => {
                        println!("New size {}x{}", width, height);
                        return Ok(cmd::TerminalResize);
                    }
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

            let mut stdout = stdout();
            execute!(stdout, event::DisableMouseCapture).expect("Failed to disable mouse capture");
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            reader: Reader::new(),
            started: false,
        }
    }

    fn start(&mut self) -> crossterm::Result<()> {
        if !self.started {
            terminal::enable_raw_mode()?;

            let mut stdout = stdout();
            execute!(stdout, event::EnableMouseCapture)?;
            self.started = true;
        }
        Ok(())
    }

    pub fn get_command(&mut self) -> crossterm::Result<UserCommand> {
        self.start()?;

        // TODO: Different keymaps for different modes. user-input, scrolling, etc.
        self.reader.get_command()
    }
}
