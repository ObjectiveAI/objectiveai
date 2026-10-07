//! Why a log could not be read or written.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// What the log store fails with: the file, or an item that would not
/// serialize or parse.
pub enum Error {
    /// A file could not be read, written or removed.
    Io {
        /// Which.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// An item would not serialize, or a line would not parse.
    Json(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Json(error) => write!(f, "a log item did not serialize: {error}"),
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
            Error::Io { source, .. } => Some(source),
            Error::Json(error) => Some(error),
        }
    }
}

impl Error {
    /// An I/O failure at `path`.
    pub fn io(path: &std::path::Path, source: io::Error) -> Self {
        Error::Io {
            path: path.to_path_buf(),
            source,
        }
    }
}
