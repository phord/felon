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
    input: Input,
    doc: Document,
}

impl Viewer {
    pub fn new(config: Config) -> Self {
        let doc = Document::new(config.clone());
        Self {
            _config: config.clone(),
            display: Display::new(config.clone()),
            status: StatusLine::new(&config),
            search: Search::new(&config),
            input: Input::new(),
            doc,
        }
    }

    // Begin owning the terminal
    pub fn start(&mut self) -> crossterm::Result<()> {
        self.display.start()
    }

    pub fn run(&mut self) -> crossterm::Result<bool> {
        self.display.refresh_screen(&mut self.doc)?;
        self.status.refresh_screen(&mut self.doc)?;

        let cmd = self.input.get_command()?;
        match cmd {
            UserCommand::None => {},
            _ => {  log::trace!("Got command: {:?}", cmd); }
        };

        match cmd {
            UserCommand::Quit => return Ok(false),
            UserCommand::ForwardSearchPrompt => self.search.prompt_forward_start()?,
            UserCommand::BackwardSearchPrompt => self.search.prompt_backward_start()?,
            _ => self.display.handle_command(cmd),
        }

        if self.search.run() {
            let srch = self.search.get_expr();
            log::trace!("Got search: {:?}", &srch);
            if self.display.set_search(srch) {
                self.display.handle_command(UserCommand::SearchNext);
            }
        }

        Ok(true)
    }

}

impl Drop for Viewer {
    fn drop(&mut self) {
        // Output::clear_screen().expect("Error");
    }
}
