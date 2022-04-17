extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;

mod indexer;

#[derive(Debug, StructOpt)]
#[structopt(name = "parser", about = "An experiment in text indexing", author = "")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
    search_word: Option<String>,
}

fn main() {
    let opt = Opt::from_args();
    let file = indexer::LogFile::new(opt.input);

    println!("{:?}", file);

    if let Some(word) = opt.search_word {
        let lines = file.index.search_word(&word);
        match lines {
            Some(lines) => {
                println!("Found {} lines for word '{}'", lines.len(), word);
                // for line in lines {
                //     println!("{}", line);
                // }
            }
            None => {
                println!("No lines found for word '{}'", word);
            }
        }
    }
}
