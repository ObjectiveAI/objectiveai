//! Why the program could not run.

use std::fmt;
use std::io;

use crate::config;

/// What [`run`](super::run) fails with: the runtime, or the
/// configuration. A `main` returning one of these prints it, and that
/// is the one report a failed run gets, so its `Debug` is its
/// `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The root, the file, or the names could not be used.
    Config(config::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(error) => write!(f, "the runtime could not be built: {error}"),
            Error::Config(error) => write!(f, "{error}"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Runtime(error) => Some(error),
            Error::Config(error) => Some(error),
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}
