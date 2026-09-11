//! A credential file that could not be read back.

use std::error;
use std::fmt;
use std::io;

/// Why the documents could not be read back after the gateway exited.
#[derive(Debug)]
pub enum ReadBackError {
    /// `auth.json` or the Qwen file could not be read.
    Io(io::Error),
    /// `auth.json` would not parse, or an entry would not
    /// re-serialize.
    Json(serde_json::Error),
}

impl fmt::Display for ReadBackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadBackError::Io(error) => {
                write!(f, "the filesystem could not be read back: {error}")
            }
            ReadBackError::Json(error) => {
                write!(f, "auth.json could not be read back: {error}")
            }
        }
    }
}

impl error::Error for ReadBackError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ReadBackError::Io(error) => Some(error),
            ReadBackError::Json(error) => Some(error),
        }
    }
}

impl From<io::Error> for ReadBackError {
    fn from(error: io::Error) -> Self {
        ReadBackError::Io(error)
    }
}

impl From<serde_json::Error> for ReadBackError {
    fn from(error: serde_json::Error) -> Self {
        ReadBackError::Json(error)
    }
}
