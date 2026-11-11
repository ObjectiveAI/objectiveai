//! Why a tool call did not do what it was asked.

use std::fmt;

/// A tool call's failure, said to the calling agent as the tool's
/// error result: the request was the board's to serve, and the agent
/// should read why it was not done.
#[derive(Debug)]
pub enum Failure {
    /// The call carried no caller the daemon attested, or one that is
    /// not an agent: the board serves agents.
    Caller(String),
    /// The tool's parameters are not what it takes.
    Parameters(String),
    /// The board holds no task by this id.
    NoTask(i64),
    /// The task is not open, so it cannot be claimed.
    NotOpen(i64),
    /// The task is not claimed by the caller, so the caller cannot
    /// complete or release it.
    NotHolder(i64),
    /// The task was posted by another agent, so the caller cannot
    /// delete it.
    NotPoster(i64),
    /// The database could not be reached, or refused a statement.
    Store(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Caller(why) => write!(f, "the board serves agents, and could not tell which agent is calling: {why}"),
            Failure::Parameters(why) => write!(f, "the parameters are not what this tool takes: {why}"),
            Failure::NoTask(id) => write!(f, "there is no task {id}"),
            Failure::NotOpen(id) => write!(f, "task {id} is not open; only an open task can be claimed"),
            Failure::NotHolder(id) => write!(f, "you do not hold task {id}; only the agent that claimed it can complete or release it"),
            Failure::NotPoster(id) => write!(f, "task {id} was posted by another agent; only its poster can delete it"),
            Failure::Store(why) => write!(f, "the board could not be read or written: {why}"),
        }
    }
}

impl std::error::Error for Failure {}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(error.to_string())
    }
}
