use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

#[derive(Default)]
pub struct InputLine { }

impl InputLine {
    pub fn run(&mut self, prompt: &str) -> Option<String> {
        let mut line_editor = Reedline::create();
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