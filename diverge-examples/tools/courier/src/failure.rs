//! Why a tool call did not do what it was asked.

use std::fmt;

/// A tool call's failure, said to the calling agent as the tool's
/// error result: the request was the courier's to serve, and the
/// agent should read why it was not done.
#[derive(Debug)]
pub enum Failure {
    /// The courier has not been registered yet, so it does not know
    /// which template's mail it keeps.
    Unregistered,
    /// The call carried no caller the daemon attested, or one that is
    /// not an agent: the courier serves agents.
    Caller(String),
    /// The tool's parameters are not what it takes.
    Parameters(String),
    /// The conversation is not one the caller is in, or does not
    /// exist.
    NoConversation(i64),
    /// The agent written to is the caller itself.
    ToSelf,
    /// The daemon names no such agent.
    NoAgent,
    /// The courier's grants do not reach the agent: it does not use
    /// this courier, and it has already run.
    Forbidden,
    /// The daemon could not be reached, or answered with an error.
    Daemon(String),
    /// The database could not be reached, or refused a statement.
    Store(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Unregistered => f.write_str("the courier has not been registered yet"),
            Failure::Caller(why) => write!(f, "the courier serves agents, and could not tell which agent is calling: {why}"),
            Failure::Parameters(why) => write!(f, "the parameters are not what this tool takes: {why}"),
            Failure::NoConversation(id) => write!(f, "you have no conversation {id}"),
            Failure::ToSelf => f.write_str("a conversation is with another agent, not with yourself"),
            Failure::NoAgent => f.write_str("the daemon has no such agent"),
            Failure::Forbidden => f.write_str("the courier's grants do not reach that agent; nothing was sent"),
            Failure::Daemon(why) => write!(f, "the daemon could not be asked: {why}; nothing was sent"),
            Failure::Store(why) => write!(f, "the mail could not be read or written: {why}"),
        }
    }
}

impl std::error::Error for Failure {}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(error.to_string())
    }
}
