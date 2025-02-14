use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crossterm::{QueueableCommand, cursor, terminal};
use crate::styled_text::styled_line::RGB_BLACK;
use crate::input_line::InputLine;

pub enum InputAction {
    None,
    Search(bool, String),
    Cancel,
}

pub struct Search {
    active: bool,
    prompt: SearchPrompt,
    forward: bool,
    expr: String,
}


impl Search {
    pub fn new(config: &Config) -> Self {
        Self {
            active: false,
            prompt: SearchPrompt::new(config),
            forward: true,
            expr: String::default(),
        }
    }

    pub fn prompt_filter_start(&mut self) -> std::io::Result<()> {
        self.forward = true;    // FIXME
        self.active = true;
        self.prompt.start("&/")
    }

    pub fn prompt_forward_start(&mut self) -> std::io::Result<()> {
        self.forward = true;
        self.active = true;
        self.prompt.start("/")
    }

    pub fn prompt_backward_start(&mut self) -> std::io::Result<()> {
        self.forward = false;
        self.active = true;
        self.prompt.start("?")
    }

    pub fn get_expr(&self) -> &str {
        &self.expr
    }

    pub fn run(&mut self) -> InputAction {
        if !self.active {
            InputAction::None
        } else {
            let input = self.prompt.run();
            if let Some(input) = input {
                self.active = false;
                // Empty input means repeat previous search
                let input = input.trim_end_matches('\r');
                if input.is_empty() {
                    if self.expr.is_empty() {
                        return InputAction::Cancel
                    }
                } else {
                    self.expr = input.to_string();
                }
                InputAction::Search(self.forward, self.expr.clone())
            } else {
                self.active = false;
                InputAction::Cancel
            }
        }
    }
}

pub struct SearchPrompt {
    color: bool,
    prompt: String,
}

impl SearchPrompt {
    pub fn new(config: &Config) -> Self {
        Self {
            color: config.color,
            prompt: String::default(),
        }
    }

    pub fn get_height(&self) -> u16 {
        1
    }

    pub fn start(&mut self, prompt: &str) -> std::io::Result<()> {
        let (_width, height) = terminal::size().expect("Unable to get terminal size");

        self.prompt = prompt.to_string();

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