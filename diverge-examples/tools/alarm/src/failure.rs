//! Why a tool call did not do what it was asked.

use std::fmt;

/// A tool call's failure, said to the calling agent as the tool's
/// error result: the request was the alarm tool's to serve, and the
/// agent should read why it was not done.
#[derive(Debug)]
pub enum Failure {
    /// The call carried no caller the daemon attested, or one that is
    /// not an agent: an alarm wakes an agent.
    Caller(String),
    /// The tool's parameters are not what it takes.
    Parameters(String),
    /// The alarm's time is not one it can be set for: none or more
    /// than one of `at`, `in_seconds` and `cron`, a time that is not
    /// RFC 3339 or is past, or a cron pattern that does not parse.
    When(String),
    /// The caller has no alarm by this id.
    NoAlarm(i64),
    /// The daemon could not be reached, or refused the message.
    Daemon(String),
    /// The database could not be reached, or refused a statement.
    Store(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Caller(why) => write!(f, "alarms are an agent's, and the tool could not tell which agent is calling: {why}"),
            Failure::Parameters(why) => write!(f, "the parameters are not what this tool takes: {why}"),
            Failure::When(why) => write!(f, "the alarm cannot be set for that time: {why}"),
            Failure::NoAlarm(id) => write!(f, "you have no alarm {id}"),
            Failure::Daemon(why) => write!(f, "the daemon could not be asked: {why}"),
            Failure::Store(why) => write!(f, "the alarms could not be read or written: {why}"),
        }
    }
}

impl std::error::Error for Failure {}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Failure::Store(error.to_string())
    }
}
