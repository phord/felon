use crate::config::Config;
use crate::display::Display;
use crate::keyboard::{Input, UserCommand};
use indexed_file::indexer::LogFile;

pub struct Viewer {
    config: Config,
    display: Display,
    input: Input,
    file: LogFile,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let filename = config.filename.get(0).expect("No filename specified").clone();
        Self {
            config: config.clone(),
            display: Display::new(config),
            input: Input::new(),
            file: LogFile::new(Some(filename)).expect("Failed to open file"),
        }
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        self.display.refresh_screen()?;

        let cmd = self.input.process_keypress()?;

        match cmd {
            UserCommand::Quit => Ok(false),
            _ => Ok(true),
        }
    }

}

impl Drop for Viewer {
    fn drop(&mut self) {
        // Output::clear_screen().expect("Error");
    }
}
