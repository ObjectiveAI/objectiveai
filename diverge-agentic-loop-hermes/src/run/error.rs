//! A run that could not be driven at all.

use std::error;
use std::fmt;
use std::io;
use std::process::ExitStatus;

use crate::filesystem::{FinishError, PrepareError};

/// Why the run never got going, or could not close.
///
/// These are the failures with nothing to salvage: before the
/// gateway is up there is no session to speak of, and a closer that
/// cannot be produced leaves nothing to resume from. Everything in
/// between — a turn that fails, a stream that breaks — is said in
/// the stream itself, as a fatal `notification`, and the run still
/// closes with what the database holds.
#[derive(Debug)]
pub enum Error {
    /// The prompt is empty: a turn needs one.
    Prompt,
    /// The filesystem could not be laid down.
    Prepare(PrepareError),
    /// `hermes gateway` could not be spawned, or stopped.
    Gateway(io::Error),
    /// The gateway exited before its API server ever answered.
    GatewayExited(ExitStatus),
    /// The way back up failed before the continuation could close
    /// the run.
    Finish(FinishError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Prompt => f.write_str("a turn needs a prompt"),
            Error::Prepare(error) => write!(f, "{error}"),
            Error::Gateway(error) => {
                write!(f, "the gateway process failed: {error}")
            }
            Error::GatewayExited(status) => {
                write!(f, "the gateway exited before it was ready: {status}")
            }
            Error::Finish(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Prepare(error) => Some(error),
            Error::Gateway(error) => Some(error),
            Error::Finish(error) => Some(error),
            Error::Prompt | Error::GatewayExited(_) => None,
        }
    }
}

impl From<PrepareError> for Error {
    fn from(error: PrepareError) -> Self {
        Error::Prepare(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Gateway(error)
    }
}

impl From<FinishError> for Error {
    fn from(error: FinishError) -> Self {
        Error::Finish(error)
    }
}
