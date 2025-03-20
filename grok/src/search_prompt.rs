use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crate::keyboard::UserCommand;
use crate::user_input::UserInput;
use crossterm::{QueueableCommand, cursor, terminal};
use crate::styled_text::styled_line::RGB_BLACK;
use crate::input_line::InputLine;

pub enum SearchPromptMode {
    Forward,
    Backward,
    Filter,
}

pub struct Search {
    prompt: SearchPrompt,
    mode: SearchPromptMode,
}

impl Search {
    pub fn new(config: &Config, mode: SearchPromptMode) -> Self {
        let prompt_string = match mode {
            SearchPromptMode::Forward => "/",
            SearchPromptMode::Backward => "?",
            SearchPromptMode::Filter => "&/",
        };

        Self {
            prompt: SearchPrompt::new(config, prompt_string),
            mode,
        }
    }
}

impl UserInput for Search {
    // Note: timeout is ignored because our string input does not timeout yet.  This is a blocking call.
    fn get_command(&mut self, _timeout: u64) -> std::io::Result<UserCommand> {
        match self.prompt.run() {
            Some(srch) => {
                let srch = srch.trim_end_matches('\r').to_string();
                match self.mode {
                    SearchPromptMode::Forward => Ok(UserCommand::ForwardSearch(srch)),
                    SearchPromptMode::Backward => Ok(UserCommand::BackwardSearch(srch)),
                    SearchPromptMode::Filter => Ok(UserCommand::Filter(srch)),
                }
            },
            None => Ok(UserCommand::Cancel),
        }
    }

    fn stop(&mut self) -> std::io::Result<()> {
        // Nothing to do
        Ok(())
    }
}

pub struct SearchPrompt {
    color: bool,
    prompt: String,
}

impl SearchPrompt {
    pub fn new(config: &Config, prompt: &str) -> Self {
        let mut sp = Self {
            color: config.color,
            prompt: prompt.to_string(),
        };
        sp.start().expect("Unable to start search prompt");
        sp
    }

    pub fn get_height(&self) -> u16 {
        1
    }

    pub fn start(&mut self) -> std::io::Result<()> {
        let (_width, height) = terminal::size().expect("Unable to get terminal size");

        let mut stdout = stdout();
        stdout.queue(cursor::MoveTo(0, height - self.get_height()))?;
        if self.color {
            // TODO: Move to Stylist?
            stdout.queue(crossterm::style::SetBackgroundColor(RGB_BLACK))?;
        }
        stdout.queue(terminal::Clear(ClearType::UntilNewLine))?;
        stdout.flush()
    }

    pub fn run(&mut self) -> Option<String> {
        let mut input_line = InputLine::default();
        input_line.run(&self.prompt)
    }

}