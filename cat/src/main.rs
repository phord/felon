use tools::cat::*;

fn main() {
    flexi_logger::Logger::try_with_env_or_str("trace").unwrap().start().unwrap();
    cat_cmd();               // 21s
    // copycat_cmd();      // 7s
    // copycat_merged_cmd();
    // cat::itercat_cmd();      // 11s
    // cat::bufcat_cmd();       // 11s
    // cat::rev_itercat_cmd();     // 16s
}
