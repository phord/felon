extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;
use std::time::{Instant};

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
    let index_timer = Instant::now();

    let file = indexer::LogFile::new(opt.input);
    println!("Index time: {}", index_timer.elapsed().as_millis() as f32 / 1000.);

    println!("{:?}", file);

    if let Some(word) = opt.search_word {
        let lookup_timer = Instant::now();
        let lines = file.search_word(&word);
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
        println!("Lookup time: {}", lookup_timer.elapsed().as_millis() as f32 / 1000.);

    }
}
