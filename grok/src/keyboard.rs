use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{event, terminal};
use std::collections::HashMap;
use std::time::Duration;

use UserCommand as cmd;
const KEYMAP: &'static [(&'static str, UserCommand)] = &[
    ("Ctrl+Q", cmd::Quit),
    ("Esc", cmd::Quit),
    ("Up", cmd::ScrollUp),
    ("Down", cmd::ScrollDown),
    ("PageUp", cmd::PageUp),
    ("PageDown", cmd::PageDown),
];

#[derive(Copy, Clone)]
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
    keymap: HashMap<KeyEvent, UserCommand>,
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
        // TODO:
        let keymap: HashMap<_, _> = KEYMAP
            .iter()
            .map(|(key, cmd)| (Self::keycode(key).unwrap(), *cmd))
            .collect();

        Self {
            reader: Reader,
            started: false,
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
        let event = self.read_key()?;
        let cmd = self.keymap.get(&event);
        match cmd {
            Some(cmd) => Ok(*cmd),
            None => Ok(UserCommand::None),
        }
    }
}
