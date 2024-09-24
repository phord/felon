use crossterm::{terminal::ClearType};
use std::{io, io::{stdout, Write}, cmp};
use crossterm::{cursor, execute, queue, terminal};
use crate::config::Config;
use crate::keyboard::UserCommand;
use crate::styled_text::{PattColor, RegionColor, StyledLine};
use crate::document::Document;
use crate::styled_text::RGB_BLACK;


#[derive(PartialEq, Debug)]
struct DisplayState {
    top: usize,     // deprecated
    bottom: usize,  // FIXME: height
    // left_offset: usize, // column offset
    width: usize,
}

struct ScreenBuffer {
    // content: String,
    content: Vec<StyledLine>,
    width: usize,
}

impl ScreenBuffer {

    fn new() -> Self {
        Self {
            content: Vec::new(),
            width: 0,
        }
    }

    fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    fn push(&mut self, line: StyledLine) {
        self.content.push(line)
    }

    fn push_raw(&mut self, data: &str) {
        self.content.push(StyledLine::new(data, PattColor::None))
    }
}

impl io::Write for ScreenBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => {
                self.push_raw(s);
                Ok(s.len())
            }
            Err(_) => Err(io::ErrorKind::WriteZero.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut buffer = String::new();
        for row in &self.content {
            let pairs = row.phrases.iter().zip(row.phrases[1..].iter());
            for (p, pnext) in pairs{
                match p.patt {
                    PattColor::None => {
                        buffer.push_str(&row.line);
                        break;
                    }
                    _ => {
                        let end = cmp::min(self.width, pnext.start);
                        assert!(end > p.start || end == 0);
                        let reg = RegionColor {len: (end - p.start) as u16, style: p.patt};
                        let content = reg.to_str(&row.line[p.start..end]);
                        buffer.push_str(content.as_str());
                        if end == self.width {
                            break;
                        }
                    }
                }
            }
        }
        let out = write!(stdout(), "{}", buffer);
        stdout().flush()?;
        self.content.clear();
        out
    }
}

pub struct Display {
    // Physical size of the display
    height: usize,
    width: usize,
    on_alt_screen: bool,
    use_alt: bool,


    /// First line on the display (line-number in the filtered file)
    top: usize,

    /// Size of the bottom status panel
    panel: usize,

    /// Previously displayed lines
    prev: DisplayState,

    // Displayed line offsets
    displayed_lines: Vec<usize>,

    mouse_wheel_height: u16,

}

impl Drop for Display {
    fn drop(&mut self) {
        log::trace!("Display closing");
        self.stop().expect("Failed to stop display");
    }
}

impl Display {
    pub fn new(config: Config) -> Self {
        let s = Self {
            height: 0,
            width: 0,
            on_alt_screen: false,
            use_alt: config.altscreen,
            top: 0,
            panel: 1,
            prev: DisplayState { top: 0, bottom: 0, width: 0},
            displayed_lines: Vec::new(),
            mouse_wheel_height: config.mouse_scroll,
        };
        s
    }

    // Begin owning the terminal
    pub fn start(&mut self) -> crossterm::Result<()> {
        if ! self.on_alt_screen && self.use_alt {
            execute!(stdout(), terminal::EnterAlternateScreen)?;
            self.on_alt_screen = true;
        }

        // Hide the cursor
        execute!(stdout(), cursor::Hide)?;

        // Collect display size info
        self.update_size();

        Ok(())
    }

    fn stop(&mut self) -> crossterm::Result<()> {
        if self.on_alt_screen {
            execute!(stdout(), terminal::LeaveAlternateScreen).expect("Failed to exit alt mode");
            self.on_alt_screen = false;
            log::trace!("display: leave alt screen");
        }

        // Show the cursor
        execute!(stdout(), cursor::Show)?;

        Ok(())
    }

    fn update_size(&mut self) {
        let (width, height) = terminal::size().expect("Unable to get terminal size");
        self.width = width as usize;
        self.height = height as usize;
    }

    fn page_size(&self) -> usize {
        cmp::max(self.height as isize - self.panel as isize, 0) as usize
    }

    fn set_status_msg(&mut self, _msg: String) {
        // FIXME
        // self.message = msg;
        // self.action = Action::Message;
    }

    fn vert_scroll(&mut self, amount: isize) {
        let top = self.top as isize + amount;
        self.top = cmp::max(top, 0) as usize;
    }


    pub fn handle_command(&mut self, cmd: UserCommand) {
        match cmd {
            UserCommand::ScrollDown => {
                self.vert_scroll(1);
            }
            UserCommand::ScrollUp => {
                self.vert_scroll(-1);
            }
            UserCommand::PageDown => {
                self.vert_scroll(self.page_size() as isize);
            }
            UserCommand::PageUp => {
                self.vert_scroll(-(self.page_size() as isize));
            }
            UserCommand::ScrollToTop => {
                self.top = 0;
            }
            UserCommand::ScrollToBottom => {
                self.top = usize::MAX;
            }
            UserCommand::TerminalResize => {
                self.update_size();
            }
            UserCommand::SelectWordAt(_x, _y) => {
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::SelectWordDrag(_x, _y) => {
                // println!("{:?}\r", cmd);
                // FIXME: Highlight the words selected
                // Add to some search struct and highlight matches
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::MouseScrollUp => {
                self.vert_scroll(-(self.mouse_wheel_height as isize));
                self.set_status_msg(format!("{:?}", cmd));
            }
            UserCommand::MouseScrollDown => {
                self.vert_scroll(self.mouse_wheel_height as isize);
                self.set_status_msg(format!("{:?}", cmd));
            }
            _ => {}
        }
    }

    fn draw_styled_line(&self, buff: &mut ScreenBuffer, row: usize, line: StyledLine) {
        queue!(buff, cursor::MoveTo(0, row as u16)).unwrap();

        buff.set_width(self.width);
        buff.push(line);

        queue!(buff, crossterm::style::SetBackgroundColor(RGB_BLACK), terminal::Clear(ClearType::UntilNewLine)).unwrap();
    }

    fn draw_line(&self, doc: &Document, buff: &mut ScreenBuffer, row: usize, line: &String) {
        // TODO: Memoize the line_colors along with the lines
        self.draw_styled_line(buff, row, doc.line_colors(line));
    }
}

#[derive(Debug)]
struct ScrollVector {
    offset: usize,  // line offset in Document
    pos: usize,     // position on screen
    lines: usize,   // number of lines to display
}

#[derive(Debug)]
enum Scroll {
    Up(ScrollVector),
    Down(ScrollVector),
    Repaint(ScrollVector),
    None,
}

impl Scroll {
    fn down(offset: usize, pos: usize, lines: usize) -> Self {
        Self::Down( ScrollVector {offset, pos, lines} )
    }
    fn up(offset: usize, pos: usize, lines: usize) -> Self {
        Self::Up( ScrollVector {offset, pos, lines} )
    }
    fn repaint(offset: usize, pos: usize, lines: usize) -> Self {
        Self::Repaint( ScrollVector {offset, pos, lines} )
    }
    fn none() -> Self {
        Self::None
    }
    fn is_none(&self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }
}

impl Display {
    // Pull lines from an iterator and display them.  There are two modes:
    // 1. Scroll up:  Display each new line at the next lower position, and scroll up from bottom
    // 2. Scroll down:  Display each new line at the next higher position, and scroll down from top
    // pos is the offset in the file for the first line
    // if size is larger than display height, we may skip unnecessary lines
    // Scroll distance is in screen rows.  If a read line takes multiple rows, they count as multiple lines.
    fn feed_lines(&mut self, doc: &mut Document, scroll: Scroll) -> crossterm::Result<ScreenBuffer> {
        log::trace!("feed_lines: {:?}", scroll);

        let mut buff = ScreenBuffer::new();

        // Check if scroll is an Up type
        let (lines, v) = match scroll {
            Scroll::Up(sv) => {
                // Partial screen scroll backwards
                let lines: Vec<_> = doc.get_lines_from_rev(sv.offset, sv.lines).into_iter().rev().collect();
                queue!(buff, terminal::ScrollDown(sv.lines as u16)).unwrap();
                self.displayed_lines.splice(0..0, lines.iter().map(|(pos, _)| *pos).take(sv.lines));
                self.displayed_lines.truncate(self.page_size());
                (lines, sv)
            },
            Scroll::Down(sv) => {
                // Partial screen scroll forwards
                let lines = doc.get_lines_from(sv.offset, sv.lines);
                queue!(buff, terminal::ScrollUp(sv.lines as u16)).unwrap();
                self.displayed_lines = self.displayed_lines[sv.lines as usize..].to_vec();
                self.displayed_lines.extend(lines.iter().map(|(pos, _)| *pos).take(sv.lines));
                // self.displayed_lines.resize(self.page_size(), 0usize);
                (lines, sv)
            },
            Scroll::Repaint(sv) => {
                // Repainting whole screen, no scrolling
                let lines = doc.get_lines_from(sv.offset, sv.lines);
                queue!(buff, terminal::Clear(ClearType::All)).unwrap();
                self.displayed_lines = lines.iter().map(|(pos, _)| *pos).take(sv.lines).collect();
                (lines, sv)
            },
            Scroll::None => unreachable!("Scroll::None")
        };

        let mut row = v.pos;
        for (pos, line) in lines.iter(){
            if row == v.pos + v.lines {
                break;
            }
            self.draw_line(doc, &mut buff, row, line);
            // self.displayed_lines[row] = *pos;
            row += 1;
        }

        Ok(buff)
    }

    pub fn refresh_screen(&mut self, doc: &mut Document) -> crossterm::Result<()> {
        // FIXME: Discard unused cached lines

        let view_height = self.page_size();

        // FIXME: We don't know last page if file isn't fully loaded yet
        let last_page = 1000; // FIXME: doc.filtered_line_count();
        self.top = cmp::min(self.top, last_page);

        // What we want to display
        let disp = DisplayState {
            top: self.top,
            bottom: self.top + view_height,
            width: self.width,
        };

        if disp == self.prev {
            // No change; nothing to do.
            return Ok(());
        }

        log::trace!("New display: {:?}", disp);
        // FIXME: We never show line 0
        // FIXME: Startup iterates whole file first
        // FIXME: Scroll to end doesn't update display; fucks up line position trackers
        // TODO: We only use displayed_lines.{first,last}.  Do we need to keep the whole array?
        // TODO: Line wrapping

        let prev_height = self.prev.bottom - self.prev.top;
        let prev_width = self.prev.width;
        let plan =
            if self.displayed_lines.len() == 0 {
                // Blank slate; start of file
                log::trace!("start of file");
                Scroll::repaint(0, 0, view_height)
            } else if self.prev.top > disp.bottom || self.prev.bottom < disp.top {
                // new screen has no overlap with old one
                if disp.top == 0 {
                    Scroll::repaint(0, 0, view_height)
                } else if disp.top == last_page {
                    // FIXME: Scrolling in last page is broken horribly
                    Scroll::up(usize::MAX, view_height-1, view_height)
                } else {
                    panic!("How did we scroll here? Not start; not end. Get offset from line number? {:?}", disp);
                    let offset = 0;
                    Scroll::repaint(offset, self.top, view_height)
                }
            } else if self.prev.top > disp.top {
                // // get 'len' lines after the top line
                // doc.get_lines_from(start + self.prev.top, len)
                // FIXME: Backwards iterator isn't working
                let begin = self.displayed_lines.first().unwrap();
                Scroll::up(*begin, 0, self.prev.top - disp.top)
            } else if self.prev.top < disp.top {
                // get 'len' lines after the last line displayed
                let begin = self.displayed_lines.last().unwrap();
                let len = disp.top - self.prev.top;
                Scroll::down(*begin, view_height - len, len)
            } else {
                // No scrolling; check height/width
                if disp.width <= prev_width {
                    if view_height <= prev_height {
                        // Screen is the same or smaller. Nothing to do.
                        Scroll::none()
                    } else {
                        // Terminal got taller; display new rows at bottom
                        let begin = self.displayed_lines.last().unwrap();
                        Scroll::down(*begin, self.prev.bottom, view_height - prev_height)
                    }
                } else {
                    // Screen got wider.  Repaint everything.
                    Scroll::repaint(0, 0, view_height)
                }
            };

        if plan.is_none() {
            return Ok(());
        }

        let mut buff = self.feed_lines(doc, plan)?;
        self.prev = disp;

        // DEBUG HACK
        // self.draw_line(doc, &mut buff, self.height - 2, &format!("scroll={} displayed={:?}", scroll, self.displayed_lines));
        buff.flush()
    }

}