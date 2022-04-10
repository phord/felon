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
    indexer::run(opt.input);
}