//! A way back up that could not be walked.

use std::error;
use std::fmt;
use std::io;

use super::PrepareError;
use super::continuation::ReadError;

/// Why the filesystem could not be streamed out.
#[derive(Debug)]
pub enum FinishError {
    /// Re-reading the request's agent failed the way preparing it
    /// would have: the wrong agent kind, a contradiction.
    Plan(PrepareError),
    /// `auth.json` or the Qwen file could not be read.
    Io(io::Error),
    /// `auth.json` would not parse, or an entry would not
    /// re-serialize.
    Json(serde_json::Error),
    /// The continuation could not be harvested.
    Continuation(ReadError),
}

impl fmt::Display for FinishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinishError::Plan(error) => write!(f, "{error}"),
            FinishError::Io(error) => {
                write!(f, "the filesystem could not be read back: {error}")
            }
            FinishError::Json(error) => {
                write!(f, "auth.json could not be read back: {error}")
            }
            FinishError::Continuation(error) => write!(f, "{error}"),
        }
    }
}

impl error::Error for FinishError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            FinishError::Plan(error) => Some(error),
            FinishError::Io(error) => Some(error),
            FinishError::Json(error) => Some(error),
            FinishError::Continuation(error) => Some(error),
        }
    }
}

impl From<PrepareError> for FinishError {
    fn from(error: PrepareError) -> Self {
        FinishError::Plan(error)
    }
}

impl From<io::Error> for FinishError {
    fn from(error: io::Error) -> Self {
        FinishError::Io(error)
    }
}

impl From<serde_json::Error> for FinishError {
    fn from(error: serde_json::Error) -> Self {
        FinishError::Json(error)
    }
}

impl From<ReadError> for FinishError {
    fn from(error: ReadError) -> Self {
        FinishError::Continuation(error)
    }
}
