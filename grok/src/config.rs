use std::path::PathBuf;
use itertools::Itertools;

pub enum ConfigItem {
    OpenFile(PathBuf),
    Chop(bool),
    AltScreen(bool),
    Color(bool),
    Visual(bool),
    MouseScroll(u16),
    // HideBefore(DateTime),
    // HideAfter(DateTime),
    // Search(String),
    // FilterIn(String),
    // FilterOut(String),
    // Style(String, PattColor),
    // Match(String, PattColor),

}

#[derive(Debug, Clone)]
pub struct Config {
    pub filename: Vec<PathBuf>,
    pub chop: bool,
    pub altscreen: bool,
    pub color: bool,
    pub version: bool,
    pub mouse: bool,
    pub mouse_scroll: u16,      // Number of lines to scroll with mouse-wheel
}

#[derive(Debug)]
pub enum Error {
    FileNotFound(String),
    ExpectedInt(String),
    ExpectedArgumentFor(String),
    UnknownArgument(String),
}

const HELP: &str = "\
App

USAGE:
  grok [OPTIONS] [INPUT ...]

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  -S --chop-long-lines  Chop long lines instead of wrapping
  -X                    Skip terminal config/cleanup such as using the alternate screen
  -C --color            Use color highlighting of parsed lines
  -V --version          Display version information

ARGS:
  <INPUT>               Input file(s) to read
";

impl Config {
    fn new() -> Self {
        Config {
            filename: Vec::new(),
            chop: false,
            altscreen: true,
            color: false,
            version: false,
            mouse: false,
            mouse_scroll: 5,
        }
    }

    /// Parse commandline arguments into config options
    pub fn from_env() -> Result<Config, Error> {
        let mut cfg = Config::new();
        cfg.parse_args()?;
        Ok(cfg)
    }

    /// Handler for each config option we know. Accumulates settings into self
    pub fn receive_item(&mut self, item: ConfigItem) {
        match item {
            ConfigItem::OpenFile(path) => self.filename.push(path),
            ConfigItem::Chop(chop) => self.chop = chop,
            ConfigItem::AltScreen(altscreen) => self.altscreen = altscreen,
            ConfigItem::Color(color) => self.color = color,
            ConfigItem::Visual(visual) => self.mouse = visual,
            ConfigItem::MouseScroll(scroll) => self.mouse_scroll = scroll,
        }
    }

    /// Parse a string argument and optionally, the word after it, into a ConfigItem.
    /// Unrecognized switches are interpreted as files to add to the queue.
    pub fn parse_item(&mut self, item: &str, arg: Option<&str>) -> Result<(ConfigItem, bool) , Error> {
        let (arg, used) = if let Some(args) = item.split_once("=") {
            (Some(args.1), false)
        } else {
            (arg, true)
        };

        let mut consumed = false;
        let cfg = match item {
            "-S" | "--chop-long-lines" => ConfigItem::Chop(!self.chop),
            "-X" | "--no-alternate-screen" => ConfigItem::AltScreen(!self.altscreen),
            "-C" | "--color" => ConfigItem::Color(!self.color),
            "-M" | "--mouse" => ConfigItem::Visual(!self.mouse),
            "-H" | "--help" => todo!("help"),
            "-V" | "--version" => todo!("version"),
            "-W" | "--wheel-lines" => {
                if let Some(arg) = arg {
                    if let Ok(num) = arg.parse::<u16>() {
                        consumed = used;
                        ConfigItem::MouseScroll(num)
                    } else {
                        return Err(Error::ExpectedInt(arg.to_string()));
                    }
                } else {
                    return Err(Error::ExpectedArgumentFor(item.to_string()));
                }
            },
            _ => ConfigItem::OpenFile(PathBuf::from(item)),
        };
        Ok((cfg, consumed))
    }



    // pub fn to_config_item();
    // TODO: Need some way to handle "toggle" values; eg., -S at runtime toggles slice
    fn parse_args(&mut self) -> Result<(), Error>{
        let mut skip = false;
        for arg_pairs in std::env::args()
                .chain(std::iter::once("".to_string()))
                .skip(1)
                .tuple_windows() {
            if skip {
                skip = false;
                continue;
            }
            let (item, arg) = arg_pairs;
            let cfg = self.parse_item(&item, Some(&arg));
            match cfg {
                Ok((item, consumed)) => {
                    self.receive_item(item);
                    skip = consumed;
                },
                Err(e) => {
                    eprintln!("Error: Parsing {:?}: {:?}", item, e);
                    return Err(e)
                }
            }
        }
        Ok(())
    }
}