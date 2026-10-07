//! Why the daemon's directory or its file could not be used.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// What [`dir`](super::dir) and [`load`](super::load) fail with: the
/// arguments, the host, the filesystem, or the file's own content.
#[derive(Debug)]
pub enum Error {
    /// An argument the daemon does not take, or a `--config` with no
    /// directory after it.
    Arguments(String),
    /// No directory was named and the host has no home directory to
    /// put one under.
    Home,
    /// A directory or a file could not be made or read.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The file does not parse, or says something the document does
    /// not take.
    Parse {
        /// The file.
        path: PathBuf,
        /// Where in it, and what was wrong.
        source: serde_path_to_error::Error<serde_yaml_ng::Error>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Arguments(argument) => {
                write!(f, "the argument `{argument}` is not one the daemon takes; `--config <dir>` is")
            }
            Error::Home => write!(f, "no directory was named and the host has no home directory"),
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Parse { path, source } => write!(f, "`{}` could not be read: {source}", path.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Arguments(_) => None,
            Error::Home => None,
            Error::Io { source, .. } => Some(source),
            Error::Parse { source, .. } => Some(source),
        }
    }
}
