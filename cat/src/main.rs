mod config;
use indexed_file::{line_indexer::LineIndexer, files::LogFile};

use config::Config;
fn main() {
    let cfg = Config::from_env();

    if cfg.is_ok() {
        for file in cfg.unwrap().filename {
            // println!("{:?}", file);
            // TODO: Teach LineIndexer to open the file by names
            let mut file = LineIndexer::new( LogFile::new_text_file(Some(file)).unwrap());
            // TODO: Open all files at once and iterate them sorted if timestamped
            // TODO: Print lines with colors
            for (line, _start, _end) in file.iter_lines() {
                println!("{line}");
            }
        }
    }
}
