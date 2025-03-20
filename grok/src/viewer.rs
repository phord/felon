use crate::config::Config;
use crate::display::Display;
use crate::status_line::StatusLine;
use crate::search_prompt::{Search, SearchPromptMode};
use crate::keyboard::{Input, UserCommand};
use crate::document::Document;
use crate::user_input::UserInput;

pub struct Viewer {
    _config: Config,
    display: Display,
    status: StatusLine,
    modalinput: Box<dyn UserInput>,
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
            modalinput: Box::new(Input::new(&config)),
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

        // FIXME: Only refresh status if modalinput is still Input
        self.status.refresh_screen(&mut self.doc)?;

        let cmd = self.modalinput.get_command(event_timeout)?;
        match cmd {
            UserCommand::None => { self.fill_timeout += 3; },
            _ => {  self.fill_timeout = 0; log::trace!("Got command: {:?}", cmd); }
        };

        match &cmd {
            UserCommand::Quit => return Ok(false),

            // Begin prompts
            UserCommand::ForwardSearchPrompt | UserCommand::BackwardSearchPrompt | UserCommand::FilterPrompt => {
                self.modalinput.stop().expect("Failed to stop modal input");
                let mode = match cmd {
                    UserCommand::ForwardSearchPrompt => SearchPromptMode::Forward,
                    UserCommand::BackwardSearchPrompt => SearchPromptMode::Backward,
                    UserCommand::FilterPrompt => SearchPromptMode::Filter,
                    _ => unreachable!(),
                };
                self.modalinput = Box::new(Search::new(&self._config, mode));
            },

            // Finish prompts
            UserCommand::ForwardSearch(srch) | UserCommand::BackwardSearch(srch) => {
                if !srch.is_empty() {
                    let fwd = matches!(cmd, UserCommand::ForwardSearch(_));
                    self.display.set_search(&mut self.doc, srch, fwd);
                }
                self.display.handle_command(UserCommand::SearchNext);
            },
            UserCommand::Filter(filt) => {
                self.display.set_filter(&mut self.doc, filt);
                self.display.handle_command(UserCommand::RefreshDisplay);
            },
            UserCommand::Cancel => {
                self.display.handle_command(UserCommand::RefreshDisplay);
            },
            _ => {},
        }

        match cmd {
            UserCommand::ForwardSearchPrompt | UserCommand::BackwardSearchPrompt | UserCommand::FilterPrompt => {
                // FIXME: Move this special-handling down into Display?
            },
            // Prompt finish cleanup
            UserCommand::Cancel | UserCommand::ForwardSearch(_) | UserCommand::BackwardSearch(_) | UserCommand::Filter(_) => {
                self.modalinput = Box::new(Input::new(&self._config));
            },

            // Forward everything else to display
            _ => self.display.handle_command(cmd),
        }

        Ok(true)
    }

}

impl Drop for Viewer {
    fn drop(&mut self) {
        // Output::clear_screen().expect("Error");
    }
}
