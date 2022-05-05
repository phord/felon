use std::path::PathBuf;
use pico_args::Arguments;

#[derive(Debug, Clone)]
pub struct Config {
    pub filename: Vec<PathBuf>,
    pub chop: bool,
    pub altscreen: bool,
    pub mouse_scroll: u16,      // Number of lines to scroll with mouse-wheel
}


const HELP: &str = "\
App

USAGE:
  pgrok [OPTIONS] [INPUT ...]

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  -S --chop-long-lines  Chop long lines instead of wrapping
  -X                    Skip terminal config/cleanup such as using the alternate screen

ARGS:
  <INPUT>               Input file(s) to read
";

impl Config {
    fn new() -> Self {
        Config {
            filename: Vec::new(),
            chop: false,
            altscreen: true,
            mouse_scroll: 5,
        }
    }

// fn main() {
//     let args = match parse_args() {
//         Ok(v) => v,
//         Err(e) => {
//             eprintln!("Error: {}.", e);
//             std::process::exit(1);
//         }
//     };

//     println!("{:#?}", args);
// }

    pub fn from_env() -> Result<Config, pico_args::Error> {
        let mut pargs = pico_args::Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            print!("{}", HELP);
            std::process::exit(0);
        }

        let mut cfg = Config::new();
        cfg.parse_args(pargs);
        Ok(cfg)
    }

    // TODO: Need some way to handle "toggle" values; eg., -S at runtime toggles slice
    fn parse_args(&mut self, mut pargs: Arguments) {
        if pargs.contains(["-S", "--chop-long-lines"]) { self.chop = true; }
        if pargs.contains(["-X", "--no-alternate-screen"]) { self.altscreen = false; }

        // Parse remaining args as input filenames
        for ostr in pargs.finish() {
            if let Some(s) = ostr.to_str() {
                if s.bytes().nth(0) == Some(b'-') {
                    eprintln!("Error: Unknown argument: {:?}", ostr);
                    std::process::exit(1);
                }
            }
            self.filename.push(PathBuf::from(ostr));
        }
    }
}