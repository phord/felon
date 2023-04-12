mod config;
mod cat;
use cat::cat_cmd;

fn main() {
    cat_cmd();               // 21s
    // cat::copycat_cmd();      // 7s
    // cat::itercat_cmd();      // 11s
    // cat::bufcat_cmd();       // 11s
    // cat::rev_itercat_cmd();     // 16s
}
