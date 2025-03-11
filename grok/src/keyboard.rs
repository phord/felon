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
use crate::config::Config;

const KEYMAP: &[(&str, UserCommand)] = &[
    ("Ctrl+W", UserCommand::Quit),
    ("Shift+Q", UserCommand::Quit),
    ("Q", UserCommand::Quit),
    ("Esc", UserCommand::Quit),
    ("Up", UserCommand::ScrollUp),
    ("Down", UserCommand::ScrollDown),
    ("Ctrl+Left", UserCommand::PanLeftMax),
    ("Ctrl+Right", UserCommand::PanRightMax),
    ("Left", UserCommand::PanLeft),
    ("Right", UserCommand::PanRight),
    ("PageUp", UserCommand::PageUp),
    ("PageDown", UserCommand::PageDown),
    ("Home", UserCommand::ScrollToTop),
    ("End", UserCommand::ScrollToBottom),
    ("&", UserCommand::FilterPrompt),
    ("/", UserCommand::ForwardSearchPrompt),
    ("?", UserCommand::BackwardSearchPrompt),
    ("N", UserCommand::SearchNext),
    ("Shift+N", UserCommand::SearchPrev),

    ("R", UserCommand::RefreshDisplay),
    ("Ctrl+R", UserCommand::RefreshDisplay),
    ("Ctrl+L", UserCommand::RefreshDisplay),
    ("Shift+R", UserCommand::RefreshDisplay),     // FIXME: and reload files

    //     PgUp b ^B ESC-v w - scroll back one page (opposite of SPACE); w is sticky
    // TODO: ESC-v?
    ("B", UserCommand::PageUp),
    ("Ctrl+B", UserCommand::PageUp),
    ("W", UserCommand::PageUpSticky),

    // PgDn SPACE ^V ^F f z -- move down one page or N lines (if N was given first); z is sticky (saves the page size)
    (" ", UserCommand::PageDown),
    ("Ctrl+V", UserCommand::PageDown),
    ("Ctrl+F", UserCommand::PageDown),
    ("F", UserCommand::PageDown),
    ("Z", UserCommand::PageDownSticky),

    // g < ESC-< - go to line N (not prompted; default 1)
    // G > ESC-> - go to line N (not prompted; default end of file)
    ("G", UserCommand::SeekStartLine),
    ("<", UserCommand::SeekStartLine),
    ("Shift+G", UserCommand::SeekEndLine),
    (">", UserCommand::SeekEndLine),

    // p - go to percentage point in file
    // P - go to byte offset in file

    ("P", UserCommand::GotoPercent),
    ("%", UserCommand::GotoPercent),
    ("Shift+P", UserCommand::GotoOffset),

    // ENTER ^N e ^E j ^J J - move down N (default 1) lines
    ("Enter", UserCommand::ScrollDown),
    ("J", UserCommand::ScrollDown),
    ("Shift+J", UserCommand::ScrollDown),
    ("Ctrl+J", UserCommand::ScrollDown),
    ("E", UserCommand::ScrollDown),
    ("Ctrl+E", UserCommand::ScrollDown),

    // y ^Y ^P k ^K K Y - scroll up N lines (opposite of j)
    // J K and Y scroll past end/begin of screen. All others stop at file edges
    ("Y", UserCommand::ScrollUp),
    ("K", UserCommand::ScrollUp),
    ("Shift+Y", UserCommand::ScrollUp),
    ("Shift+K", UserCommand::ScrollUp),
    ("Ctrl+Y", UserCommand::ScrollUp),
    ("Ctrl+P", UserCommand::ScrollUp),
    ("Ctrl+K", UserCommand::ScrollUp),

    // d ^D - scroll forward half a screen or N lines; N is sticky; becomes new default for d/u
    // u ^U - scroll up half a screen or N lines; N is sticky; becomes new default for d/u
    ("D", UserCommand::HalfPageDown),
    ("Ctrl+D", UserCommand::HalfPageDown),
    ("U", UserCommand::HalfPageUp),
    ("Ctrl+U", UserCommand::HalfPageUp),

    // F - go to end of file and try to read more data
    ("Shift+F", UserCommand::SeekEndLine),        // TODO: and read more data

    // m <x> - bookmark first line on screen with letter given (x is any alpha, upper or lower)
    // M <x> - bookmark last line on screen with letter given
    // ' <x> - go to bookmark with letter given (and position as it was marked, at top or bottom)
    // ^X^X <n> - got to bookmark
    ("M", UserCommand::SetBookmarkTop),
    ("Shift+M", UserCommand::SetBookmarkBottom),
    ("'", UserCommand::GotoBookmark),
    ("Ctrl+X", UserCommand::GotoBookmark),

    // Digits: accumulate a number argument for the next command
    ("0", UserCommand::CollectDigits(0)),
    ("1", UserCommand::CollectDigits(1)),
    ("2", UserCommand::CollectDigits(2)),
    ("3", UserCommand::CollectDigits(3)),
    ("4", UserCommand::CollectDigits(4)),
    ("5", UserCommand::CollectDigits(5)),
    ("6", UserCommand::CollectDigits(6)),
    ("7", UserCommand::CollectDigits(7)),
    ("8", UserCommand::CollectDigits(8)),
    ("9", UserCommand::CollectDigits(9)),
    (".", UserCommand::CollectDecimal),

    // Dash preceeds an option
    ("-", UserCommand::ChordKey('-', 2)),

    // Mouse action mappings
    // Note that if any mouse mappings are enabled, the code will turn on MouseTrap mode in the terminal. This
    // affects how the mouse is used. In particular, highlighting text, copy and paste functions from the terminal
    // probably won't work as they normally do.  We can't emulate those features either since we don't have access
    // to the user's clipboard unless we're on the same X server.

    ("MouseLeft", UserCommand::SelectWordAt(0,0)),
    ("MouseLeftDrag", UserCommand::SelectWordDrag(0,0)),
    // ("Ctrl+MouseLeft", UserCommand::ScrollDown),
    // ("MouseRight", UserCommand::MouseRight),
    // ("MouseMiddle", UserCommand::MouseMiddle),
    ("MouseWheelUp", UserCommand::MouseScrollUp),
    ("MouseWheelDown", UserCommand::MouseScrollDown),

];

#[derive(Clone, Debug)]
pub enum UserCommand {
    None,
    BackwardSearchPrompt,
    FilterPrompt,
    ForwardSearchPrompt,
    HalfPageDown,
    HalfPageUp,
    GotoBookmark,
    SetBookmarkTop,
    SetBookmarkBottom,
    GotoOffset,
    GotoPercent,
    SeekStartLine,
    SeekEndLine,
    CollectDigits(u8),
    CollectDecimal,
    MouseScrollDown,
    MouseScrollUp,
    PageDown,
    PageDownSticky,
    PageUp,
    PageUpSticky,
    Quit,
    RefreshDisplay,
    ScrollDown,
    ScrollToBottom,
    ScrollToTop,
    ScrollUp,
    PanLeft,
    PanRight,
    PanLeftMax,
    PanRightMax,
    SearchNext,
    SearchPrev,
    SelectWordAt(u16, u16),
    SelectWordDrag(u16, u16),
    TerminalResize,
    ChordKey(char, u8),
    Chord(String),
}

// TODO: Roll this into a test
// use crossterm::event::{Event, KeyCode, MouseButton, KeyEvent, MouseEvent, MouseEventKind, KeyModifiers};
// use lgt::keyboard::Reader;
// assert_eq!(Reader::keycode("Ctrl+Q"), KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));

#[derive(Default)]
struct Reader {
    keymap: HashMap<KeyEvent, UserCommand>,
    mousemap: HashMap<MouseEvent, UserCommand>,
    chord: String,
    chord_len: u8,
}

impl Reader {

    pub fn new() -> Self {
        let allmap: HashMap<_, _> = KEYMAP
            .iter()
            .map(|(key, cmd)| (Self::keycode(key).unwrap(), cmd.clone()))
            .collect();

        let keymap: HashMap<_, _> = allmap.iter()
            .filter(|(event, _)| matches!(event, Event::Key(_)) )
            .map(|(event, cmd)| match event { Event::Key(key_event) => (*key_event, cmd.clone()), _ => unreachable!() })
            .collect();

        let mousemap: HashMap<_, _> = allmap.iter()
            .filter(|(event, _)| matches!(event, Event::Mouse(_)) )
            .map(|(event, cmd)| match event { Event::Mouse(mouse_event) => (*mouse_event, cmd.clone()), _ => unreachable!() })
            .collect();

        Self {
            keymap,
            mousemap,
            chord: String::new(),
            chord_len: 0,
        }
    }

    /// Convert a string representation of a key combo into a Key or Mouse Event
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

    pub fn reset_chord(&mut self) {
        self.chord_len = 0;
    }

    fn get_command(&mut self, timeout: u64) -> std::io::Result<UserCommand> {
        if !event::poll(Duration::from_millis(timeout))? {
            Ok(UserCommand::None)
        } else {
            match event::read()? {
                Event::Key(event) => {
                    if self.chord_len > 0 {
                        // FIXME: Chord should collect full key string (i.e. "Ctrl+K", "Meta+Home")
                        self.chord.push_str(&format!("{}", event.code));
                        self.chord_len -= 1;
                        if self.chord_len == 0 {
                            Ok(UserCommand::Chord(self.chord.clone()))
                        } else {
                            Ok(UserCommand::None)
                        }
                    } else {
                        Ok(match self.keymap.get(&event) {
                            Some(UserCommand::ChordKey(key, len)) => {
                                self.chord = key.to_string();
                                self.chord_len = len - 1;
                                UserCommand::None
                            },
                            Some(cmd) => {
                                cmd.clone()
                            },
                            None => UserCommand::None,
                        })
                    }
                },
                Event::FocusGained | Event::FocusLost | Event::Paste(_) => {
                    Ok(UserCommand::None)
                },
                Event::Mouse(event) => {
                    let lookup = MouseEvent {
                        column:0, row:0,
                        ..event
                    };

                    // println!("{:?}", event);

                    match self.mousemap.get(&lookup) {
                        Some(cmd) => {
                            match cmd {
                                UserCommand::SelectWordAt(_,_) => {
                                    Ok(UserCommand::SelectWordAt(event.column, event.row))
                                },
                                UserCommand::SelectWordDrag(_,_) => {
                                    Ok(UserCommand::SelectWordDrag(event.column, event.row))
                                },
                                _ => Ok(cmd.clone()),
                            }
                        },
                        None => Ok(UserCommand::None),
                    }
                }
                Event::Resize(_, _) => {
                    Ok(UserCommand::TerminalResize)
                }
            }
        }
    }

}

#[derive(Default)]
pub struct Input {
    reader: Reader,
    started: bool,
    mouse: bool,
}

impl Drop for Input {
    fn drop(&mut self) {
        if self.started {
            terminal::disable_raw_mode().expect("Unable to disable raw mode");

            let mut stdout = stdout();
            if self.mouse {
                execute!(stdout, event::DisableMouseCapture).expect("Failed to disable mouse capture");
            }
        }
    }
}

impl Input {
    pub fn new(config: &Config) -> Self {
        Self {
            reader: Reader::new(),
            started: false,
            mouse: config.mouse,
        }
    }

    fn start(&mut self) -> std::io::Result<()> {
        if !self.started {
            terminal::enable_raw_mode()?;

            let mut stdout = stdout();
            if self.mouse {
                execute!(stdout, event::EnableMouseCapture)?;
            }
            self.started = true;
        }
        Ok(())
    }

    pub fn get_command(&mut self, timeout: u64) -> std::io::Result<UserCommand> {
        self.start()?;

        // TODO: Different keymaps for different modes. user-input, scrolling, etc.
        self.reader.get_command(timeout)
    }
    pub fn reset_chord(&mut self) {
        self.reader.reset_chord();
    }
}
