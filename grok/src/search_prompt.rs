use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crossterm::{QueueableCommand, cursor, terminal};
use crate::styled_text::styled_line::RGB_BLACK;
use crate::input_line::InputLine;

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

    pub fn prompt_filter_start(&mut self) -> crossterm::Result<()> {
        self.forward = true;    // FIXME
        self.active = true;
        self.prompt.start("&/")
    }

    pub fn prompt_forward_start(&mut self) -> crossterm::Result<()> {
        self.forward = true;
        self.active = true;
        self.prompt.start("/")
    }

    pub fn prompt_backward_start(&mut self) -> crossterm::Result<()> {
        self.forward = false;
        self.active = true;
        self.prompt.start("?")
    }

    pub fn get_expr(&self) -> &str {
        &self.expr
    }

    pub fn run(&mut self) -> bool {
        if !self.active { false }
        else {
            let input = self.prompt.run();
            if let Some(input) = input {
                self.active = false;
                // Empty input means repeat previous search
                let input = input.trim_end_matches('\r');
                if input.is_empty() {
                    return !self.expr.is_empty()
                }
                self.expr = input.to_string();
                true
            } else { false }
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

    pub fn start(&mut self, prompt: &str) -> crossterm::Result<()> {
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