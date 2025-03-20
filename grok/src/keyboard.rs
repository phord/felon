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
use crate::user_input::UserInput;

const BASE_KEYMAP: &[(&str, UserCommand)] = &[
    ("Ctrl+W", UserCommand::Quit),
    ("Shift+Q", UserCommand::Quit),
    ("Q", UserCommand::Quit),
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
];

// Additional keys for "less" compatibility
const LESS_KEYMAP: &[(&str, UserCommand)] = &[
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
    ("Space", UserCommand::PageDown),
    ("Ctrl+V", UserCommand::PageDown),
    ("Ctrl+F", UserCommand::PageDown),
    ("F", UserCommand::PageDown),
    ("Z", UserCommand::PageDownSticky),

    // g < ESC-< - go to line N (not prompted; default 1)
    ("G", UserCommand::SeekStartLine),
    ("<", UserCommand::SeekStartLine),
    ("Esc <", UserCommand::SeekStartLine),

    // G > ESC-> - go to line N (not prompted; default end of file)
    ("Shift+G", UserCommand::SeekEndLine),
    (">", UserCommand::SeekEndLine),
    ("Esc >", UserCommand::SeekEndLine),

    ("Esc Shift+G", UserCommand::SeekEndLine),        // TODO: Different from Shift+G, which should keep searching on stdin
    ("Esc Shift+F", UserCommand::SeekEndLine),        // TODO: Stop on pattern match

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
    ("-", UserCommand::SwitchToggle),
    ("_", UserCommand::SwitchDescribe),

    // ESC chords
    ("Esc )", UserCommand::PanRight),
    ("Esc (", UserCommand::PanLeft),
    ("Esc }", UserCommand::PanRightMax),
    ("Esc {", UserCommand::PanLeftMax),
    ("Esc V", UserCommand::PageUp),

    // ("Esc Ctrl+F [:print:] [:print:]", UserCommand::NextMatchingBraceCustom),
    // ("Esc Ctrl+B [:print:] [:print:]", UserCommand::PrevMatchingBraceCustom),
    // ("Esc M [A-Za-z]", UserCommand::MarkClear),
    // ("Esc /", UserCommand::SearchNextFiles),
    // ("Esc ?", UserCommand::SearchPrevFiles),
    // ("Esc N", UserCommand::SearchNextFiles),
    // ("Esc Shift+N", UserCommand::SearchPrevFiles),
    // ("Esc U", UserCommand::DisableSearchHighlight),
    // ("Esc Shift+U", UserCommand::SearchClear),

    // ("' [A-Za-z'^$]", UserCommand::MarkGoto),        // Goto named mark, or "Previous", "Top" or "Bottom"
    // ("m [A-Za-z]", UserCommand::MarkTop),
    // ("Shift+M [A-Za-z]", UserCommand::MarkBottom),

    // ("Ctrl+X Ctrl+X [A-Za-z'^$]", UserCommand::MarkGoto),        // Goto named mark, or "Previous", "Top" or "Bottom"
    // ("Ctrl+X Ctrl+V", UserCommand::AddFile),

    // ("Shift+E", UserCommand::AddFile),

    // (": E", UserCommand::AddFile),
    // ("Shift+E", UserCommand::AddFile),
    // (": N", UserCommand::NextFile),
    // (": P", UserCommand::PrevFile),
    // (": X", UserCommand::GotoFile),
    // (": D", UserCommand::RemoveFile),

    // ("=", UserCommand::ShowInfo),
    // ("Ctrl+G", UserCommand::ShowInfo),
    // (": Shift+F", UserCommand::ShowInfo),

    (": Q", UserCommand::Quit),
    (": Shift+Q", UserCommand::Quit),
    ("Shift+Z Shift+Z", UserCommand::Quit),

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

#[derive(Clone, Debug, PartialEq)]
pub enum UserCommand {
    None,
    PartialChord,  // New variant for partial chord matches
    BackwardSearchPrompt,
    FilterPrompt,
    ForwardSearchPrompt,
    BackwardSearch(String),
    Filter(String),
    ForwardSearch(String),
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
    Cancel,     // Cancel the current input mode
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
    SwitchToggle,
    SwitchDescribe,
    Chord(String),      // FIXME: Deprecated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    #[test]
    fn test_keycode_combinations() {
        let test_cases = [
            ("Ctrl+Q", Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL))),
            ("Shift+N", Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::SHIFT))),
            ("Ctrl+Shift+PageUp", Event::Key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL | KeyModifiers::SHIFT))),
        ];

        for (input, expected) in test_cases {
            let result = Reader::keycode(input).unwrap();
            assert_eq!(result, expected, "Testing key combo: {}", input);
        }
    }

    #[test]
    fn test_keycodes_combinations() {
        let test_cases = [
            ("- - -", vec![
                Event::Key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE)),
            ]),
            ("Ctrl+Q Shift+N", vec![
                Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
                Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::SHIFT)),
            ]),
            ("Esc Shift+G", vec![
                Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                Event::Key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT)),
            ]),
            ("MouseLeft MouseWheelUp", vec![
                Event::Mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 0, row: 0, modifiers: KeyModifiers::NONE }),
                Event::Mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: 0, row: 0, modifiers: KeyModifiers::NONE }),
            ]),
        ];

        for (input, expected) in test_cases {
            let result = Reader::keycodes(input).unwrap();
            assert_eq!(result, expected, "Testing key sequence: {}", input);
        }
    }

    #[test]
    fn test_keymap_construction() {
        let reader = Reader::new();
        let test_cases = [
            ("Q", UserCommand::Quit),
            ("Esc", UserCommand::Quit),
            ("MouseWheelUp", UserCommand::MouseScrollUp),
            ("MouseWheelDown", UserCommand::MouseScrollDown),
            ("Esc V", UserCommand::PageUp),
            ("Esc >", UserCommand::SeekEndLine),
        ];

        for (key_str, expected_cmd) in test_cases {
            let events = Reader::keycodes(key_str).unwrap();
            match reader.keymap.get(&events) {
                Some(cmd) => assert_eq!(cmd, &expected_cmd, "Testing keymap entry: {}", key_str),
                None => panic!("Keymap missing entry for: {}", key_str),
            }
        }
    }

    #[test]
    fn test_extend_keymap() {
        let mut reader = Reader::new();
        let extensions = [
            ("Alt+X", UserCommand::Quit),
            ("Ctrl+C", UserCommand::Quit),
            ("Alt+V", UserCommand::PageUp),
        ];

        reader.extend_keymap(&extensions);

        // Test both original and extended mappings
        let test_cases = [
            // Original mappings should still work
            ("Q", UserCommand::Quit),
            ("Esc", UserCommand::Quit),
            ("MouseWheelUp", UserCommand::MouseScrollUp),
            // New mappings should work too
            ("Alt+X", UserCommand::Quit),
            ("Ctrl+C", UserCommand::Quit),
            ("Alt+V", UserCommand::PageUp),
        ];

        for (key_str, expected_cmd) in test_cases {
            let events = Reader::keycodes(key_str).unwrap();
            match reader.keymap.get(&events) {
                Some(cmd) => assert_eq!(cmd, &expected_cmd, "Testing keymap entry: {}", key_str),
                None => panic!("Keymap missing entry for: {}", key_str),
            }
        }
    }

    #[test]
    fn test_chord_sequences_with_partial() {
        let mut reader = Reader::new();
        let chord_mappings = [
            ("Ctrl+X Ctrl+X", UserCommand::Quit),
            ("Ctrl+X Ctrl+C", UserCommand::ScrollToTop),
        ];

        reader.extend_keymap(&chord_mappings);

        // Test partial chord matches
        let partial = Reader::keycodes("Ctrl+X").unwrap();
        match reader.keymap.get(&partial) {
            Some(UserCommand::PartialChord) => (), // Success
            _ => panic!("Expected PartialChord for Ctrl+X prefix"),
        }

        // Test full chord matches
        let full_chord = Reader::keycodes("Ctrl+X Ctrl+X").unwrap();
        match reader.keymap.get(&full_chord) {
            Some(UserCommand::Quit) => (), // Success
            _ => panic!("Expected Quit for full Ctrl+X Ctrl+X chord"),
        }

        // Test non-matching sequence
        let non_match = Reader::keycodes("Ctrl+X Ctrl+V").unwrap();
        assert!(!reader.keymap.contains_key(&non_match), "Expected no match for invalid chord");
    }

    #[test]
    fn test_chord_sequence_step_by_step() {
        let mut reader = Reader::default();
        let chord_mappings = [
            ("Ctrl+X Ctrl+X", UserCommand::Quit),
            ("Ctrl+X Ctrl+C", UserCommand::ScrollToTop),
            ("g g", UserCommand::ScrollToTop),
        ];
        reader.extend_keymap(&chord_mappings);

        // Test Ctrl+X Ctrl+X sequence
        let ctrl_x = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(reader.process_event(ctrl_x.clone()), UserCommand::PartialChord, "First Ctrl+X should return PartialChord");
        assert_eq!(reader.process_event(ctrl_x.clone()), UserCommand::Quit, "Second Ctrl+X should return Quit");

        // Test failed sequence Ctrl+X Z
        assert_eq!(reader.process_event(ctrl_x.clone()), UserCommand::PartialChord, "Ctrl+X should return PartialChord");
        let key_z = Event::Key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert_eq!(reader.process_event(key_z), UserCommand::None, "Z after Ctrl+X should return None");

        // Test 'g g' sequence
        let key_g = Event::Key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(reader.process_event(key_g.clone()), UserCommand::PartialChord, "First g should return PartialChord");
        assert_eq!(reader.process_event(key_g.clone()), UserCommand::ScrollToTop, "Second g should return ScrollToTop");

        // Test reset_chord
        assert_eq!(reader.process_event(ctrl_x.clone()), UserCommand::PartialChord, "Ctrl+X should return PartialChord");
        reader.reset_chord();
        assert_eq!(reader.process_event(key_g), UserCommand::PartialChord, "After reset, g should return PartialChord");
        // Clear up the chord collector from the previous test.
        reader.reset_chord();

        // Test that non-key events don't interrupt chord collection
        assert_eq!(reader.process_event(ctrl_x.clone()), UserCommand::PartialChord, "First Ctrl+X should return PartialChord");
        assert_eq!(reader.process_event(Event::FocusGained), UserCommand::None, "FocusGained should return None");
        assert_eq!(reader.process_event(Event::FocusLost), UserCommand::None, "FocusLost should return None");
        assert_eq!(reader.process_event(Event::Paste("test".to_string())), UserCommand::None, "Paste should return None");
        assert_eq!(reader.process_event(ctrl_x.clone()), UserCommand::Quit, "Second Ctrl+X should still complete the chord");
    }

    #[test]
    fn test_invalid_key_combinations() {
        let test_cases = [
            // Duplicate modifiers
            ("Ctrl+Ctrl+X", "Key combo Ctrl+Ctrl+X gives ctrl twice"),
            ("Shift+Shift+A", "Key combo Shift+Shift+A gives shift twice"),
            ("Alt+Alt+B", "Key combo Alt+Alt+B gives alt twice"),

            // Multiple action keys
            ("A+B", "Key combo A+B has two action keys"),
            ("Enter+Space", "Key combo Enter+Space has two action keys"),
            ("Tab+Esc", "Key combo Tab+Esc has two action keys"),

            // Mixed mouse and keyboard
            ("MouseLeft+A", "Key combo MouseLeft+A has an action key and a mouse action"),
            ("Enter+MouseWheelUp", "Key combo Enter+MouseWheelUp has an action key and a mouse action"),

            // Multiple mouse actions
            ("MouseLeft+MouseRight", "Key combo MouseLeft+MouseRight has two mouse actions"),
            ("MouseWheelUp+MouseWheelDown", "Key combo MouseWheelUp+MouseWheelDown has two mouse actions"),

            // Unknown/invalid key names
            ("Foo", "Unknown key name foo in Foo"),
            ("Ctrl+Bar", "Unknown key name bar in Ctrl+Bar"),
            ("Alt+Shift+Baz", "Unknown key name baz in Alt+Shift+Baz"),

            // No action key
            ("Ctrl+Alt+Shift", "Key combo Ctrl+Alt+Shift has no action key or mouse action"),
        ];

        for (input, expected_error) in test_cases {
            match Reader::keycode(input) {
                Ok(_) => panic!("Expected error for invalid combo: {}", input),
                Err(error) => assert_eq!(error, expected_error, "Testing invalid combo: {}", input),
            }
        }
    }
}

#[derive(Default)]
struct Reader {
    keymap: HashMap<Vec<Event>, UserCommand>,
    event_sequence: Vec<Event>,
}

impl Reader {

    pub fn new() -> Self {
        let mut s = Self::default();
        s.extend_keymap(BASE_KEYMAP);
        s.extend_keymap(LESS_KEYMAP);
        s
    }

    pub fn default() -> Self {
        Self {
            keymap: Self::build_keymap(&[]),
            event_sequence: Vec::new(),
        }
    }

    fn build_keymap(mappings: &[(&str, UserCommand)]) -> HashMap<Vec<Event>, UserCommand> {
        let mut keymap = HashMap::new();

        for (key_str, cmd) in mappings {
            let events = match Self::keycodes(key_str) {
                Ok(events) => events,
                Err(e) => {
                    log::error!("Error parsing key combo: {}", e);
                    continue;
                }
            };

            // For sequences with multiple events, add prefix entries
            if events.len() > 1 {
                for i in 1..events.len() {
                    let prefix = events[0..i].to_vec();
                    if keymap.contains_key(&prefix) && keymap[&prefix] != UserCommand::PartialChord {
                        log::warn!("Keymap conflict: {:?}", prefix);
                        continue;
                    }
                    keymap.entry(prefix).or_insert(UserCommand::PartialChord);
                }
            }

            if keymap.contains_key(&events) && keymap[&events] == UserCommand::PartialChord {
                log::warn!("Keymap conflict: {:?}", &events);
            }

            // Add the full sequence
            keymap.insert(events, cmd.clone());
        }

        keymap
    }

    pub fn extend_keymap(&mut self, mappings: &[(&str, UserCommand)]) {
        self.keymap.extend(Self::build_keymap(mappings));
    }

    // Convert a string representation of a series of key combos into a Vector of Key and/or Mouse Events
    fn keycodes(orig: &str) -> Result<Vec<Event>, String> {
        let mut events = Vec::new();
        for key in orig.split(" ") {
            match Self::keycode(key) {
                Ok(event) => events.push(event),
                Err(e) => return Err(format!("Error parsing key combo {orig} at {key}: {e}")),
            }
        }
        Ok(events)
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
                "space" => Some(KeyCode::Char(' ')),
                k => {
                    if k.len() == 1 {
                        Some(KeyCode::Char(k.chars().next().unwrap()))
                    } else if k.len() > 1 && k.starts_with("F") && k.len() < 4 {
                        // FIXME: handle errors
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
                if mouse_button.is_some() {
                    return Err(format!("Key combo {} has an action key and a mouse action", orig));
                }
                action_key = action;
            } else if mouse_action.is_some() {
                // Already got a mouse action
                if mouse_button.is_some() {
                    return Err(format!("Key combo {} has two mouse actions", orig));
                }
                // Already got an action key
                if action_key.is_some() {
                    return Err(format!("Key combo {} has an action key and a mouse action", orig));
                }
                mouse_button = mouse_action;
            } else {
                return Err(format!("Unknown key name {} in {}", key, orig));
            }
        }

        if let Some(key) = action_key {
            Ok(Event::Key(KeyEvent::new(key, modifiers)))
        } else if let Some(button) = mouse_button {
            Ok(Event::Mouse(MouseEvent { kind:button, column:0, row:0, modifiers } ))
        } else {
            Err(format!("Key combo {} has no action key or mouse action", orig))
        }
    }

    pub fn reset_chord(&mut self) {
        self.event_sequence.clear();
    }

    fn process_event(&mut self, event: Event) -> UserCommand {
        let mut x = 0;
        let mut y = 0;

        // Filter out non-key/mouse events
        match event {
            Event::Key(_) => self.event_sequence.push(event),
            Event::Mouse(event) => {
                self.event_sequence.push(Event::Mouse(MouseEvent { column: 0, row: 0, ..event }));
                x = event.column;
                y = event.row;
            },
            Event::Resize(_, _) => return UserCommand::TerminalResize,
            // Ignore other events without interrupting chord collection
            _ => return UserCommand::None,
        }

        match self.keymap.get(&self.event_sequence) {
            Some(UserCommand::PartialChord) => UserCommand::PartialChord,
            Some(cmd) => {
                let result = match cmd {
                    UserCommand::SelectWordAt(_, _) => UserCommand::SelectWordAt(x, y),
                    UserCommand::SelectWordDrag(_, _) => UserCommand::SelectWordDrag(x, y),
                    _ => cmd.clone(),
                };
                self.event_sequence.clear();
                result
            },
            None => {
                self.event_sequence.clear();
                UserCommand::None
            }
        }
    }

    fn get_command(&mut self, timeout: u64) -> std::io::Result<UserCommand> {
        if !event::poll(Duration::from_millis(timeout))? {
            return Ok(UserCommand::None);
        }
        let event = event::read()?;
        Ok(self.process_event(event))
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
        self.stop().unwrap_or_default();
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

            if self.mouse {
                let mut stdout = stdout();
                execute!(stdout, event::EnableMouseCapture)?;
            }
            self.started = true;
        }
        Ok(())
    }

    pub fn reset_chord(&mut self) {
        self.reader.reset_chord();
    }
}

impl UserInput for Input {
    fn stop(&mut self) -> std::io::Result<()> {
        if self.started {
            terminal::disable_raw_mode()?;

            if self.mouse {
                let mut stdout = stdout();
                execute!(stdout, event::DisableMouseCapture)?;
            }
        }
        self.started = false;
        Ok(())
    }

    fn get_command(&mut self, timeout: u64) -> std::io::Result<UserCommand> {
        self.start()?;

        // TODO: Different keymaps for different modes. user-input, scrolling, etc.
        self.reader.get_command(timeout)
    }
}
