// TODO Investigate using xterm control codes to manipulate the clipboard
// https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
// https://github.com/microsoft/terminal/issues/2946#issuecomment-626355734

// OSC 52 doesn't work for me.  It's not supported in Konsole.
// https://bugs.kde.org/show_bug.cgi?id=372116
// https://bugzilla.gnome.org/show_bug.cgi?id=795774

// https://cirw.in/blog/bracketed-paste (quasi-related)
// http://www.xfree86.org/current/ctlseqs.html
// https://unix.stackexchange.com/questions/16694/copy-input-to-clipboard-over-ssh

use crossterm::event::{Event, KeyCode, MouseButton, KeyEvent, MouseEvent, MouseEventKind, KeyModifiers};
use crossterm::{event, terminal, execute};
use std::collections::HashMap;
use std::time::Duration;
use std::io::stdout;

use UserCommand as cmd;
const KEYMAP: &[(&str, UserCommand)] = &[
    ("Ctrl+W", cmd::Quit),
    ("Q", cmd::Quit),
    ("Esc", cmd::Quit),
    ("Up", cmd::ScrollUp),
    ("Down", cmd::ScrollDown),
    ("PageUp", cmd::PageUp),
    ("PageDown", cmd::PageDown),
    ("Home", cmd::ScrollToTop),
    ("End", cmd::ScrollToBottom),
    ("&", cmd::FilterPrompt),
    ("/", cmd::ForwardSearchPrompt),
    ("?", cmd::BackwardSearchPrompt),
    ("n", cmd::SearchNext),
    ("N", cmd::SearchPrev),

    // Mouse action mappings
    // Note that if any mouse mappings are enabled, the code will turn on MouseTrap mode in the terminal. This
    // affects how the mouse is used. In particular, highlighting text, copy and paste functions from the terminal
    // probably won't work as they normally do.  We can't emulate those features either since we don't have access
    // to the user's clipboard unless we're on the same X server.

    ("MouseLeft", cmd::SelectWordAt(0,0)),
    ("MouseLeftDrag", cmd::SelectWordDrag(0,0)),
    // ("Ctrl+MouseLeft", cmd::ScrollDown),
    // ("MouseRight", cmd::MouseRight),
    // ("MouseMiddle", cmd::MouseMiddle),
    ("MouseWheelUp", cmd::MouseScrollUp),
    ("MouseWheelDown", cmd::MouseScrollDown),

];

#[derive(Copy, Clone, Debug)]
pub enum UserCommand {
    None,
    Quit,
    ScrollUp,
    ScrollDown,
    MouseScrollUp,
    MouseScrollDown,
    PageUp,
    PageDown,
    ScrollToTop,
    ScrollToBottom,
    TerminalResize,
    RefreshDisplay,
    FilterPrompt,
    ForwardSearchPrompt,
    BackwardSearchPrompt,
    SearchNext,
    SearchPrev,
    SelectWordAt(u16, u16),
    SelectWordDrag(u16, u16),
}

struct Reader {
    keymap: HashMap<KeyEvent, UserCommand>,
    mousemap: HashMap<MouseEvent, UserCommand>,
}

impl Reader {

    pub fn new() -> Self {
        let allmap: HashMap<_, _> = KEYMAP
            .iter()
            .map(|(key, cmd)| (Self::keycode(key).unwrap(), *cmd))
            .collect();

        let keymap: HashMap<_, _> = allmap.iter()
            .filter(|(event, _)| matches!(event, Event::Key(_)) )
            .map(|(event, cmd)| match event { Event::Key(key_event) => (*key_event, *cmd), _ => unreachable!() })
            .collect();

        let mousemap: HashMap<_, _> = allmap.iter()
            .filter(|(event, _)| matches!(event, Event::Mouse(_)) )
            .map(|(event, cmd)| match event { Event::Mouse(mouse_event) => (*mouse_event, *cmd), _ => unreachable!() })
            .collect();

        Self {
            keymap,
            mousemap,
        }
    }

    /// Convert a string representation of a key combo into a Key or Mouse Event
    /// ```
    /// assert_eq!(crate::Input::keycode("Ctrl+Q"), KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    /// ```
    fn keycode(orig: &str) -> Result<Event, String> {
        let mut modifiers = KeyModifiers::NONE;
        let mut action_key: Option<KeyCode> = None;
        let mut mouse_button: Option<MouseEventKind> = None;

        let str = orig.to_lowercase();
        for key in str.split("+") {
            let mods = match key {
                "shift" => crossterm::event::KeyModifiers::SHIFT,
                "alt" => crossterm::event::KeyModifiers::ALT,
                "ctrl" => crossterm::event::KeyModifiers::CONTROL,
                _ => crossterm::event::KeyModifiers::NONE,
            };

            let action = match key {
                "backspace" => Some(KeyCode::Backspace),
                "enter" => Some(KeyCode::Enter),
                "left" => Some(KeyCode::Left),
                "right" => Some(KeyCode::Right),
                "up" => Some(KeyCode::Up),
                "down" => Some(KeyCode::Down),
                "home" => Some(KeyCode::Home),
                "end" => Some(KeyCode::End),
                "pageup" => Some(KeyCode::PageUp),
                "pagedown" => Some(KeyCode::PageDown),
                "tab" => Some(KeyCode::Tab),
                "backtab" => Some(KeyCode::BackTab),
                "delete" => Some(KeyCode::Delete),
                "insert" => Some(KeyCode::Insert),
                "null" => Some(KeyCode::Null),
                "esc" => Some(KeyCode::Esc),
                k => {
                    if k.len() == 1 {
                        Some(KeyCode::Char(k.chars().next().unwrap()))
                    } else if k.len() > 1 && k.starts_with("F") && k.len() < 4 {
                        Some(KeyCode::F(k[1..].parse().unwrap()))
                    } else {
                        None
                    }
                }
            };

            let mouse_action = match key {
                "mouseleft" => Some(MouseEventKind::Down(MouseButton::Left)),
                "mouseleftup" => Some(MouseEventKind::Up(MouseButton::Left)),
                "mouseleftdrag" => Some(MouseEventKind::Drag(MouseButton::Left)),
                "mouseright" => Some(MouseEventKind::Down(MouseButton::Right)),
                "mouserightup" => Some(MouseEventKind::Up(MouseButton::Right)),
                "mouserightdrag" => Some(MouseEventKind::Drag(MouseButton::Right)),
                "mousemiddle" => Some(MouseEventKind::Down(MouseButton::Middle)),
                "mousemiddleup" => Some(MouseEventKind::Up(MouseButton::Middle)),
                "mousemiddledrag" => Some(MouseEventKind::Drag(MouseButton::Middle)),
                "mousewheelup" => Some(MouseEventKind::ScrollUp),
                "mousewheeldown" => Some(MouseEventKind::ScrollDown),
                _ => None,
            };

            if mods != KeyModifiers::NONE {
                if modifiers & mods != KeyModifiers::NONE {
                    return Err(format!("Key combo {} gives {} twice", orig, key));
                }
                modifiers |= mods;
            } else if action.is_some() {
                // Already got an action key
                if action_key.is_some() {
                    return Err(format!("Key combo {} has two action keys", orig));
                }
                if mouse_action.is_some() {
                    return Err(format!("Key combo {} has an action key and a mouse action", orig));
                }
                action_key = action;
            } else if mouse_action.is_some() {
                // Already got a mouse action
                if mouse_button.is_some() {
                    return Err(format!("Key combo {} has two mouse actions", orig));
                }
                mouse_button = mouse_action;
            } else {
                return Err(format!("Unknown key name {} in {}", key, orig));
            }
        }

        assert_ne!(action_key.is_some(), mouse_button.is_some());

        if let Some(key) = action_key {
            Ok(Event::Key(KeyEvent::new(key, modifiers)))
        } else if let Some(button) = mouse_button {
            Ok(Event::Mouse(MouseEvent { kind:button, column:0, row:0, modifiers } ))
        } else {
            Err(format!("Key combo {} has no action key or mouse action", orig))
        }
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
                    Event::FocusGained | Event::FocusLost | Event::Paste(_) => {},
                    Event::Mouse(event) => {
                        let lookup = MouseEvent {
                            column:0, row:0,
                            ..event
                        };

                        // println!("{:?}", event);

                        return match self.mousemap.get(&lookup) {
                            Some(cmd) => {
                                match cmd {
                                    cmd::SelectWordAt(_,_) => {
                                        Ok(cmd::SelectWordAt(event.column, event.row))
                                    },
                                    cmd::SelectWordDrag(_,_) => {
                                        Ok(cmd::SelectWordDrag(event.column, event.row))
                                    },
                                    _ => Ok(*cmd),
                                }
                            },
                            None => Ok(UserCommand::None),
                        };
                    }
                    Event::Resize(_, _) => {
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
            if ! self.reader.mousemap.is_empty() {
                execute!(stdout, event::DisableMouseCapture).expect("Failed to disable mouse capture");
            }
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
            if ! self.reader.mousemap.is_empty() {
                execute!(stdout, event::EnableMouseCapture)?;
            }
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
