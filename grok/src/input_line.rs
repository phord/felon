use std::path::PathBuf;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use reedline::{DefaultPrompt, DefaultPromptSegment, FileBackedHistory, Reedline, Signal};
use {
    reedline::{KeyCode, KeyModifiers},
    reedline::{default_emacs_keybindings, Emacs, ReedlineEvent},
  };

#[derive(Default)]
pub struct InputLine { }

// FIXME: Make this a config option
const HISTORY_FILE: &str = "search_history";

impl InputLine {
    pub fn run(&mut self, prompt: &str) -> Option<String> {

        lazy_static! {
            static ref HISTORY_PATH: PathBuf =
                if let Some(proj_dirs) = ProjectDirs::from("com", "Phord Software", "Felon") {
                    let mut dir = proj_dirs.config_dir().to_path_buf();
                    dir.push(HISTORY_FILE);
                    log::trace!("History path: {:?}", dir);
                    dir
                } else {
                    // FIXME: Make this a hidden file?
                    PathBuf::from(HISTORY_FILE)
                };
            }

        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Esc,
            ReedlineEvent::CtrlC,
        );
        let edit_mode = Box::new(Emacs::new(keybindings));

        let history = Box::new(
          FileBackedHistory::with_file(500, HISTORY_PATH.to_path_buf())
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
