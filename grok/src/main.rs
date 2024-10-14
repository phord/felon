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
    let cfg = Config::from_env();
    log::info!("Init config: {:?}", cfg);

    let mut viewer = viewer::Viewer::new(cfg.unwrap());
    viewer.start()?;

    while viewer.run()? {}

    log::info!("main exit");
    Ok(())
}
