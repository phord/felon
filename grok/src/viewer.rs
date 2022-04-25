use crate::config::Config;
use crate::display::Display;
use crate::keyboard::{Input, UserCommand};

pub struct Viewer {
    config: Config,
    display: Display,
    input: Input,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        Self {
            config: config.clone(),
            display: Display::new(config),
            input: Input::new(),
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
