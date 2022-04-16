extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;

mod indexer;

#[derive(Debug, StructOpt)]
#[structopt(name = "parser", about = "An experiment in text indexing", author = "")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    let index = indexer::index_file(opt.input);
    println!("Indexed words: {}", index.words.len());
    println!("Indexed numbers: {}", index.numbers.len());
    println!("Total lines: {}", index.lines());
    println!("Total bytes: {}", index.bytes());
}
