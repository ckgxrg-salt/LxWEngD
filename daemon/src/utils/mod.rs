pub mod ipc;
pub mod playlist;
pub mod state;

use thiserror::Error;

#[derive(Debug, PartialEq, Error)]
pub enum ParseError {
    /// Indicates that this line is not a recognised command.
    /// Blank lines are also treated as invalid commands.
    #[error("Unrecognised command")]
    CommandNotFound,
    /// The command requires some arguments, but not enough are provided.
    /// Note that if you use `#` to comment in the line, anything after that `#` will be ignored.
    #[error("Not enough arguments")]
    NotEnoughArguments,
    /// The command requires an argument of a specific type, but the given one cannot be parsed
    /// into that type.
    #[error("Invalid arguments")]
    InvalidArgument,
}
