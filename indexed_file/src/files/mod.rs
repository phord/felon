mod mock_log_file;
mod text_log_file;
mod log_file;

pub use mock_log_file::MockLogFile;
pub use text_log_file::TextLogFile;
pub use log_file::LogFile;
pub use log_file::LogFileTrait;
