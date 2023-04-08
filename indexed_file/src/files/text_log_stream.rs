// Reader of unseekable text streams
// For a stream we have to store old lines in RAM to be able to seek around.

use crate::files::CachedStreamReader;

pub type TextLogStream = CachedStreamReader;
