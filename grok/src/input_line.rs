use reedline::{DefaultPrompt, DefaultPromptSegment, FileBackedHistory, Reedline, Signal};
use {
    reedline::{KeyCode, KeyModifiers},
    reedline::{default_emacs_keybindings, Emacs, ReedlineEvent},
  };

#[derive(Default)]
pub struct InputLine { }

// FIXME: Put this in a config path
// FIXME: Make this a config option
const HISTORY_FILE: &str = ".grok_history";

impl InputLine {
    pub fn run(&mut self, prompt: &str) -> Option<String> {

        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Esc,
            ReedlineEvent::CtrlC,
        );
        let edit_mode = Box::new(Emacs::new(keybindings));

        let history = Box::new(
          FileBackedHistory::with_file(500, HISTORY_FILE.into())
            .expect("Error configuring history with file"),
        );

        let mut line_editor = Reedline::create()
            .with_history(history)
            .with_edit_mode(edit_mode);
        let prompt = DefaultPrompt {
                left_prompt: DefaultPromptSegment::Basic(prompt.to_string()),
                .. DefaultPrompt::default()
            };
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                Some(buffer)
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                None
            }
            x => {
                log::info!("reedline Event: {:?}", x);
                None
            }
        }
    }
}