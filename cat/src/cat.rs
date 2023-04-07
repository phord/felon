use crate::config::Config;
use indexed_file::{line_indexer::LineIndexer, files::LogFile};

pub fn cat_cmd() {
    let cfg = Config::from_env();

    if cfg.is_ok() {
        let cfg = cfg.unwrap();
        let mut files:Vec<Option<_>> = cfg.filename.iter().cloned().map(|file| Some(file)).collect();
        if files.is_empty() {
            files.push(None);
        };
        for file in files {
            // println!("{:?}", file);

            // TODO: Teach LineIndexer to open the file by names and construct the correct type
            let mut file = LineIndexer::new( LogFile::new_text_file(file).unwrap());
            // TODO: Open all files at once and iterate them sorted if timestamped
            // TODO: Print lines with colors
            for (line, _start, _end) in file.iter_lines() {
                println!("{line}");
            }
        }
    }
}
