use crate::config::Config;
use crate::display::Display;
use crate::status_line::StatusLine;
use crate::search_prompt::{InputAction, Search};
use crate::keyboard::{Input, UserCommand};
use crate::document::Document;

pub struct Viewer {
    _config: Config,
    display: Display,
    status: StatusLine,
    search: Search,
    filter: Search,
    input: Input,
    doc: Document,
    fill_timeout: u64,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let doc = Document::new(config.clone());
        Self {
            _config: config.clone(),
            display: Display::new(config.clone()),
            status: StatusLine::new(&config),
            search: Search::new(&config),
            filter: Search::new(&config),
            input: Input::new(&config),
            doc,
            fill_timeout: 0,
        }
    }

    // Begin owning the terminal
    pub fn start(&mut self) -> std::io::Result<()> {
        self.display.start()
    }

    pub fn run(&mut self) -> std::io::Result<bool> {

        let event_timeout =
            if self.doc.has_pending()  {
                if let Some(offset) = self.doc.run(self.fill_timeout.min(40)) {
                    // FIXME: Update status line with Searching... / Not found.
                    self.display.goto(offset);
                    return Ok(true);
                } else {
                    0
                }
            } else {
                500
            };

        self.display.refresh_screen(&mut self.doc)?;
        self.status.refresh_screen(&mut self.doc)?;

        let cmd = self.input.get_command(event_timeout)?;
        match cmd {
            UserCommand::None => { self.fill_timeout += 3; },
            _ => {  self.fill_timeout = 0; log::trace!("Got command: {:?}", cmd); }
        };

        match cmd {
            UserCommand::Quit => return Ok(false),
            UserCommand::ForwardSearchPrompt => self.search.prompt_forward_start()?,
            UserCommand::BackwardSearchPrompt => self.search.prompt_backward_start()?,
            UserCommand::FilterPrompt => self.filter.prompt_filter_start()?,
            _ => self.display.handle_command(cmd),
        }

        match self.search.run() {
            InputAction::None => {},
            InputAction::Search(forward, srch) => {
                log::trace!("Got search: fwd={}  {:?}", forward, &srch);
                // Empty input means repeat previous search
                if !srch.is_empty() {
                    self.display.set_search(&mut self.doc, &srch, forward);
                }
                self.display.handle_command(UserCommand::SearchNext);
            },
            InputAction::Cancel => {
                log::trace!("Cancel search");
                self.display.handle_command(UserCommand::RefreshDisplay);
            },
        }

        match self.filter.run() {
            InputAction::None => {},
            InputAction::Search(_, filt) => {
                log::trace!("Got filter: {:?}", &filt);
                // Empty input means cancel all filters
                if filt.is_empty() {
                    self.display.clear_filter(&mut self.doc);
                } else {
                    self.display.set_filter(&mut self.doc, &filt);
                }
                self.display.handle_command(UserCommand::RefreshDisplay);
            },
            InputAction::Cancel => {
                log::trace!("Cancel filter");
                self.display.handle_command(UserCommand::RefreshDisplay);
            },
        }

        Ok(true)
    }

}

impl Drop for Viewer {
    fn drop(&mut self) {
        // Output::clear_screen().expect("Error");
    }
}
