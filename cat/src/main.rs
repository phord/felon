use tools::cat::*;

fn main() {
    flexi_logger::Logger::try_with_env_or_str("trace").unwrap().start().unwrap();
    cat_cmd();               // 31s
    // copycat_cmd();           // 1.4s
    // copycat_merged_cmd();    // 158s
    // itercat_cmd();           // 57s
    // brcat_cmd();           // 57s
    // bufcat_cmd();            // ???s gave up after 8 minutes  (.zst only)
    // rev_itercat_cmd();       // ???  did not test
}

// 13239317
// cargo run --release --bin cat   10.48s user 21.71s system 102% cpu 31.428 total
// 31000000 / 13239317 = 2.34us per line