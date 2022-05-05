use crate::config::Config;
use crate::display::Display;
use crate::keyboard::{Input, UserCommand};
use crate::document::Document;

pub struct Viewer {
    config: Config,
    display: Display,
    input: Input,
    doc: Document,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let doc = Document::new(config.clone());
        Self {
            config: config.clone(),
            display: Display::new(config),
            input: Input::new(),
            doc,
        }
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {

        self.display.refresh_screen(&mut self.doc)?;

        let cmd = self.input.get_command()?;

        match cmd {
            UserCommand::Quit => Ok(false),
            UserCommand::SearchPrompt => {
                self.display.search_prompt();
                Ok(true)
            }
            _ => {
                self.display.handle_command(cmd);
                Ok(true)
            }
        }
    }

}

impl Drop for Viewer {
    fn drop(&mut self) {
        // Output::clear_screen().expect("Error");
    }
}
