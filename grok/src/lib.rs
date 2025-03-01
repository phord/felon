pub mod config;
pub mod viewer;
pub mod display;
pub mod keyboard;
pub mod styled_text;
pub mod document;
pub mod status_line;
pub mod search_prompt;
pub mod input_line;

use config::Config;
use viewer::Viewer;

pub fn run() -> std::io::Result<()> {
    let cfg = Config::from_env().unwrap();

    // Check if no files given and no stdin redirection. Abort if so.
    if cfg.filename.is_empty() && std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        eprintln!("Error: No input files or pipe given");
        std::process::exit(1);
    }

    let mut viewer = Viewer::new(cfg);
    viewer.start()?;

    while viewer.run()? {}

    Ok(())
}