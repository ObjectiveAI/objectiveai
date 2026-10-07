//! Why content could not be received, placed or read.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_sdk::provider::endpoints::volumes::write::client::channel_response::FrameError;

/// What the content store fails with.
#[derive(Debug)]
pub enum Error {
    /// A file or a directory could not be made, written, read, moved
    /// or removed.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// A path the request named is not one: a component empty, `.` or
    /// `..`, a path named twice, a file under a file, or none at all.
    Path(String),
    /// A content channel ended in the client's error, or without a
    /// finish: the upload is abandoned.
    Abandoned(String),
    /// A frame on a content channel was not a content frame.
    Channel(FrameError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Path(path) => write!(f, "`{path}` is not a path a resource may hold"),
            Error::Abandoned(path) => write!(f, "the content of `{path}` did not arrive whole"),
            Error::Channel(error) => write!(f, "a content frame could not be read: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Channel(error) => Some(error),
            Error::Path(_) | Error::Abandoned(_) => None,
        }
    }
}
