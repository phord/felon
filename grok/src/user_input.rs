use std::io;
use crate::keyboard::UserCommand;

/// Modal user input handler trait.  Usually we react to immediate keyboard commands (PgUp, etc.) using
/// Reader.  But some commands require sub-handlers to prompt the user for specific info without scanning
/// for all other commands.  For example, search_prompt when the user presses '/'.
/// The UserInput trait allows us to switch between modes.
///
pub trait UserInput {
    fn get_command(&mut self, timeout: u64) -> io::Result<UserCommand>;
}