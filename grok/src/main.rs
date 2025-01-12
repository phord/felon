// mod editor;
mod config;
mod viewer;
mod display;
mod keyboard;
mod styled_text;
mod document;
mod status_line;
mod search_prompt;
use flexi_logger::{Logger, FileSpec};

use config::Config;
fn main() -> crossterm::Result<()> {
    Logger::try_with_env_or_str("trace").unwrap()
        .log_to_file(FileSpec::default().directory("/tmp"))
        .start().unwrap();
    let cfg = Config::from_env().unwrap();
    log::info!("Init config: {:?}", cfg);

    if cfg.version {
        println!("grok version 0.1.0");
        std::process::exit(0);
    }

    let mut viewer = viewer::Viewer::new(cfg);
    viewer.start()?;

    while viewer.run()? {}

    log::info!("main exit");
    Ok(())
}
