//! Why a path a request named could not be used.

use std::fmt;

/// What the path rule fails with.
#[derive(Debug)]
pub enum Error {
    /// A path the request named is not one: a component empty, `.` or
    /// `..`, a path named twice, a file under a file, or none at all.
    Path(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Path(path) => write!(f, "`{path}` is not a path a request may name"),
        }
    }
}

impl std::error::Error for Error {}
