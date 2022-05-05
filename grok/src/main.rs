// mod editor;
mod config;
mod viewer;
mod display;
mod keyboard;
mod styled_text;
mod document;

use config::Config;
fn main() -> crossterm::Result<()> {
    let cfg = Config::from_env();
    println!("{:?}", cfg);
    let mut viewer = viewer::Viewer::new(cfg.unwrap());
    while viewer.run()? {}
    Ok(())
}
