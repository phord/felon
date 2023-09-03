use tools::cat::*;
use flexi_logger::Logger; //,{AdaptiveFormat,default_format, FileSpec};

fn main() {
    Logger::try_with_env_or_str("trace").unwrap().start().unwrap();
    tac_cmd();
}
