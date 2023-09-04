use crate::config::Config;
use crate::display::Display;
use crate::status_line::StatusLine;
use crate::keyboard::{Input, UserCommand};
use crate::document::Document;

pub struct Viewer {
    _config: Config,
    display: Display,
    status: StatusLine,
    input: Input,
    doc: Document,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let doc = Document::new(config.clone());
        Self {
            _config: config.clone(),
            display: Display::new(config.clone()),
            status: StatusLine::new(config),
            input: Input::new(),
            doc,
        }
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        log::trace!("run");

        self.display.refresh_screen(&mut self.doc)?;
        self.status.refresh_screen(&mut self.doc)?;

        let cmd = self.input.get_command()?;
        log::trace!("Got command: {:?}", cmd);
        match cmd {
            UserCommand::Quit => Ok(false),
            UserCommand::SearchPrompt => {
                // FIXME self.status.search_prompt();
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
