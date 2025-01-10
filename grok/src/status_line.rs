use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crossterm::{QueueableCommand, cursor, terminal, style, style::Stylize};
use crate::document::Document;
use crate::styled_text::RGB_BLACK;

pub struct StatusLine {
    color: bool,
}

impl StatusLine {
    pub fn new(config: &Config) -> Self {
        Self {
            color: config.color,
        }
    }

    #[allow(dead_code)]
    pub fn get_height(&self) -> u16 {
        1
    }

    pub fn refresh_screen(&mut self, doc: &mut Document) -> crossterm::Result<()> {
        let (width, height) = terminal::size().expect("Unable to get terminal size");

        // FIXME: Don't print the status line again if nothing changed

        let mut stdout = stdout();
        let message =
            std::iter::once(format!("Bytes: {}", doc.len()))
            .chain(doc.info()
                .map(|stats| {
                    let indexed = stats.bytes_indexed as f64 / doc.len() as f64 * 100.0;
                    format!("{}: {} lines, {:3.2}% indexed", stats.name, stats.lines_indexed, indexed)
                })
            )
            .collect::<Vec<_>>()
            .join(" | ");

        let width = std::cmp::min(width as usize, message.len());
        stdout.queue(cursor::MoveTo(0, height-1_u16))?;
        stdout.queue(style::PrintStyledContent(message[0..width].reverse()))?;
        if self.color {
            stdout.queue(crossterm::style::SetBackgroundColor(RGB_BLACK))?;
        }
        stdout.queue(terminal::Clear(ClearType::UntilNewLine))?;

        stdout.flush()
    }

}