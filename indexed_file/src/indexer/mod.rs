pub(crate) mod line_indexer;
pub mod eventual_index;
pub mod index;
pub mod sane_index;
pub mod sane_lines;
pub mod sane_iterator;
pub mod indexed_log;
pub mod sane_indexer;
pub(crate) mod waypoint;

pub use line_indexer::LineIndexer;
pub use indexed_log::{IndexedLog, LogLocation, LineOption};