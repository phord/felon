use tools::cat::*;

/** FIXME:  First line after delay goes missing
    $ cargo run --release --bin cat <(seq 2; sleep 2 ; seq 2)
    1
    2
    2
 */
fn main() {
    flexi_logger::Logger::try_with_env_or_str("trace").unwrap().start().unwrap();
    cat_cmd();               // 21s
    // cat::copycat_cmd();      // 7s
    // cat::itercat_cmd();      // 11s
    // cat::bufcat_cmd();       // 11s
    // cat::rev_itercat_cmd();     // 16s
}
