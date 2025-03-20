use crate::keyboard::UserCommand;

/// Represents a modal input handler that takes user input and produces user commands.
///
/// # Lifecycle
/// - There is exactly one active input mode
/// - The old mode should be stopped before the new mode is started
/// - Commands are polled via get_command()
/// - Should be be stopped via `stop()` before exiting or switching to another mode
///
/// # Implementation Notes
/// - `get_command` may or may not respect the timeout parameter
/// - Implementations should document their timeout behavior
pub trait UserInput {
    /// Poll user input and return a command.  If no input is available, return UserCommand::None.
    fn get_command(&mut self, timeout: u64) -> std::io::Result<UserCommand>;

    /// Stops the current input mode and performs any necessary cleanup.
    /// Called before switching to another input mode.
    fn stop(&mut self) -> std::io::Result<()>;
}
