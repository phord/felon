use tools::cat::*;

fn main() {
    flexi_logger::Logger::try_with_env_or_str("trace").unwrap().start().unwrap();
    tail_cmd();
}
