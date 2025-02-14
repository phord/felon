use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use {
    reedline::{KeyCode, KeyModifiers},
    reedline::{default_emacs_keybindings, Emacs, ReedlineEvent},
  };

#[derive(Default)]
pub struct InputLine { }


impl InputLine {
    pub fn run(&mut self, prompt: &str) -> Option<String> {

        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Esc,
            ReedlineEvent::CtrlC,
        );
        let edit_mode = Box::new(Emacs::new(keybindings));

        let mut line_editor = Reedline::create().with_edit_mode(edit_mode);
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