use flexi_logger::{Logger, FileSpec};

fn main() -> crossterm::Result<()> {
    Logger::try_with_env_or_str("trace").unwrap()
        .log_to_file(FileSpec::default().directory("/tmp"))
        .start().unwrap();

    grok::run()
}