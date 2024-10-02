use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crossterm::{QueueableCommand, cursor, terminal, style, style::Stylize};
use crate::document::Document;
use crate::styled_text::RGB_BLACK;

pub struct StatusLine {
}

impl StatusLine {
    pub fn new(_config: Config) -> Self {
        Self { }
    }

    pub fn get_height(&self) -> u16 {
        1
    }

    pub fn refresh_screen(&mut self, _doc: &mut Document) -> crossterm::Result<()> {
        let (width, height) = terminal::size().expect("Unable to get terminal size");

        let mut stdout = stdout();

        let message = format!("Line {} of {}", "??", "??");
        // let message = format!("Showing {} of {} lines, {} filtered",
        //                               doc.filtered_line_count(), doc.all_line_count(),
        //                               doc.all_line_count() - doc.filtered_line_count());


        let width = std::cmp::min(width as usize, message.len());
        stdout.queue(cursor::MoveTo(0, height-1 as u16))?;
        stdout.queue(style::PrintStyledContent(message[0..width].reverse()))?;
        // stdout.queue(crossterm::style::SetBackgroundColor(RGB_BLACK))?;
        stdout.queue(terminal::Clear(ClearType::UntilNewLine))?;

        stdout.flush()
    }

}