//! Why a call or a page's request did not do what it was asked.

use std::fmt;

/// A failure, said to the calling agent as the tool's error result, or
/// to the page as the answer's body.
#[derive(Debug)]
pub enum Failure {
    /// The caller is not who this asks for: an agent on a tool call, a
    /// person on a frontend request.
    Caller(String),
    /// The tool's parameters, or the page's request, are not what it
    /// takes.
    Parameters(String),
    /// The daemon could not be reached, or refused.
    Daemon(String),
    /// The database could not be reached, or refused a statement.
    Store(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Caller(why) => write!(f, "the caller is not one the chat serves here: {why}"),
            Failure::Parameters(why) => write!(f, "the parameters are not what this takes: {why}"),
            Failure::Daemon(why) => write!(f, "the daemon could not be asked: {why}"),
            Failure::Store(why) => write!(f, "the messages could not be read or written: {why}"),
        }
    }
}

impl std::error::Error for Failure {}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(error.to_string())
    }
}
