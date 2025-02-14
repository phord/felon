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
    ("Shift+Q", cmd::Quit),
    ("Q", cmd::Quit),
    ("Esc", cmd::Quit),
    ("Up", cmd::ScrollUp),
    ("Down", cmd::ScrollDown),
    ("Ctrl+Left", cmd::PanLeftMax),
    ("Ctrl+Right", cmd::PanRightMax),
    ("Left", cmd::PanLeft),
    ("Right", cmd::PanRight),
    ("PageUp", cmd::PageUp),
    ("PageDown", cmd::PageDown),
    ("Home", cmd::ScrollToTop),
    ("End", cmd::ScrollToBottom),
    ("&", cmd::FilterPrompt),
    ("/", cmd::ForwardSearchPrompt),
    ("?", cmd::BackwardSearchPrompt),
    ("N", cmd::SearchNext),
    ("Shift+N", cmd::SearchPrev),

    ("R", cmd::RefreshDisplay),
    ("Ctrl+R", cmd::RefreshDisplay),
    ("Ctrl+L", cmd::RefreshDisplay),
    ("Shift+R", cmd::RefreshDisplay),     // FIXME: and reload files

    //     PgUp b ^B ESC-v w - scroll back one page (opposite of SPACE); w is sticky
    // TODO: ESC-v?
    ("B", cmd::PageUp),
    ("Ctrl+B", cmd::PageUp),
    ("W", cmd::PageUpSticky),

    // PgDn SPACE ^V ^F f z -- move down one page or N lines (if N was given first); z is sticky (saves the page size)
    (" ", cmd::PageDown),
    ("Ctrl+V", cmd::PageDown),
    ("Ctrl+F", cmd::PageDown),
    ("F", cmd::PageDown),
    ("Z", cmd::PageDownSticky),

    // g < ESC-< - go to line N (not prompted; default 1)
    // G > ESC-> - go to line N (not prompted; default end of file)
    ("G", cmd::SeekStartLine),
    ("<", cmd::SeekStartLine),
    ("Shift+G", cmd::SeekEndLine),
    (">", cmd::SeekEndLine),

    // p - go to percentage point in file
    // P - go to byte offset in file

    ("P", cmd::GotoPercent),
    ("%", cmd::GotoPercent),
    ("Shift+P", cmd::GotoOffset),

    // ENTER ^N e ^E j ^J J - move down N (default 1) lines
    ("Enter", cmd::ScrollDown),
    ("J", cmd::ScrollDown),
    ("Shift+J", cmd::ScrollDown),
    ("Ctrl+J", cmd::ScrollDown),
    ("E", cmd::ScrollDown),
    ("Ctrl+E", cmd::ScrollDown),

    // y ^Y ^P k ^K K Y - scroll up N lines (opposite of j)
    // J K and Y scroll past end/begin of screen. All others stop at file edges
    ("Y", cmd::ScrollUp),
    ("K", cmd::ScrollUp),
    ("Shift+Y", cmd::ScrollUp),
    ("Shift+K", cmd::ScrollUp),
    ("Ctrl+Y", cmd::ScrollUp),
    ("Ctrl+P", cmd::ScrollUp),
    ("Ctrl+K", cmd::ScrollUp),

    // d ^D - scroll forward half a screen or N lines; N is sticky; becomes new default for d/u
    // u ^U - scroll up half a screen or N lines; N is sticky; becomes new default for d/u
    ("D", cmd::HalfPageDown),
    ("Ctrl+D", cmd::HalfPageDown),
    ("U", cmd::HalfPageUp),
    ("Ctrl+U", cmd::HalfPageUp),

    // F - go to end of file and try to read more data
    ("Shift+F", cmd::SeekEndLine),        // TODO: and read more data

    // m <x> - bookmark first line on screen with letter given (x is any alpha, upper or lower)
    // M <x> - bookmark last line on screen with letter given
    // ' <x> - go to bookmark with letter given (and position as it was marked, at top or bottom)
    // ^X^X <n> - got to bookmark
    ("M", cmd::SetBookmarkTop),
    ("Shift+M", cmd::SetBookmarkBottom),
    ("'", cmd::GotoBookmark),
    ("Ctrl+X", cmd::GotoBookmark),

    // Digits: accumulate a number argument for the next command
    ("0", cmd::CollectDigits(0)),
    ("1", cmd::CollectDigits(1)),
    ("2", cmd::CollectDigits(2)),
    ("3", cmd::CollectDigits(3)),
    ("4", cmd::CollectDigits(4)),
    ("5", cmd::CollectDigits(5)),
    ("6", cmd::CollectDigits(6)),
    ("7", cmd::CollectDigits(7)),
    ("8", cmd::CollectDigits(8)),
    ("9", cmd::CollectDigits(9)),
    (".", cmd::CollectDecimal),

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
}

// TODO: Roll this into a test
// use crossterm::event::{Event, KeyCode, MouseButton, KeyEvent, MouseEvent, MouseEventKind, KeyModifiers};
// use grok::keyboard::Reader;
// assert_eq!(Reader::keycode("Ctrl+Q"), KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));

#[derive(Default)]
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

    fn get_command(&self, timeout: u64) -> std::io::Result<UserCommand> {
        loop {
            if !event::poll(Duration::from_millis(timeout))? {
                return Ok(UserCommand::None);
            } else {
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

#[derive(Default)]
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

    fn start(&mut self) -> std::io::Result<()> {
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

    pub fn get_command(&mut self, timeout: u64) -> std::io::Result<UserCommand> {
        self.start()?;

        // TODO: Different keymaps for different modes. user-input, scrolling, etc.
        self.reader.get_command(timeout)
    }
}
