use crate::config::Config;
use crate::display::Display;
use crate::status_line::StatusLine;
use crate::search_prompt::Search;
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
            input: Input::new(),
            doc,
            fill_timeout: 0,
        }
    }

    // Begin owning the terminal
    pub fn start(&mut self) -> crossterm::Result<()> {
        self.display.start()
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {

        let event_timeout =
            if self.fill_timeout > 0 && self.doc.fill_gaps(self.fill_timeout.min(40)) {
                0
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

        if self.search.run() {
            let srch = self.search.get_expr();
            log::trace!("Got search: {:?}", &srch);
            if !srch.is_empty() {
                self.display.set_search(&mut self.doc, srch);
            }
            self.display.handle_command(UserCommand::SearchNext);
        }

        if self.filter.run() {
            let filt = self.filter.get_expr();
            log::trace!("Got filter: {:?}", &filt);
            self.display.set_filter(&mut self.doc, filt);
            self.display.handle_command(UserCommand::RefreshDisplay);
        }

        Ok(true)
    }

}

impl Drop for Viewer {
    fn drop(&mut self) {
        // Output::clear_screen().expect("Error");
    }
}
