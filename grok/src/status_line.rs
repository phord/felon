use crossterm::terminal::ClearType;
use std::io::{stdout, Write};
use crate::config::Config;
use crossterm::{QueueableCommand, cursor, terminal, style, style::{Color, Stylize}};
use crate::document::Document;

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

        // status line:   curr_line of total_lines | "search": hit of total (hidden) | "filter": hit of total (hidden)
        let mut stdout = stdout();
        let message =
            std::iter::once(format!("Bytes: {}  {}", doc.len(), doc.describe_pending()))
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
            // TODO: Stylist?
            let fixme_inverse = Color::Rgb{r:0xc0,g:0xc0,b:0xc0}; // FIXME: use PattColor::Inverse() somehow
            stdout.queue(crossterm::style::SetBackgroundColor(fixme_inverse))?;
        }
        stdout.queue(terminal::Clear(ClearType::UntilNewLine))?;

        stdout.flush()
    }

}