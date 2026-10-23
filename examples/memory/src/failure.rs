//! Why a tool call did not do what it was asked.

use std::fmt;

/// A tool call's failure, said to the calling agent as the tool's
/// error result: the request was the tool's to serve, and the agent
/// should read why it was not done.
#[derive(Debug)]
pub enum Failure {
    /// The tool's parameters are not what it takes.
    Parameters(String),
    /// The notebook holds no note by this id.
    NoNote(i64),
    /// The database could not be reached, or refused a statement.
    Store(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Parameters(why) => write!(f, "the parameters are not what this tool takes: {why}"),
            Failure::NoNote(id) => write!(f, "there is no note {id}"),
            Failure::Store(why) => write!(f, "the notes could not be read or written: {why}"),
        }
    }
}

impl std::error::Error for Failure {}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(error.to_string())
    }
}
