//! Why the cluster could not be made, or its password had.

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;

/// What [`password`](super::password) and [`init`](super::init) fail
/// with.
#[derive(Debug)]
pub enum Error {
    /// A file or a directory could not be read, made, renamed or
    /// written.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// `<dir>/password` exists and holds nothing but whitespace.
    EmptyPassword(PathBuf),
    /// `initdb` could not be started.
    Spawn(io::Error),
    /// `initdb` ran and failed, and this is what it wrote to stderr.
    Initdb {
        /// How it exited.
        status: ExitStatus,
        /// What it wrote to stderr, whole.
        stderr: String,
    },
    /// `initdb` did not finish in time.
    Timeout,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::EmptyPassword(path) => write!(f, "`{}` holds no password", path.display()),
            Error::Spawn(error) => write!(f, "initdb could not be started: {error}"),
            Error::Initdb { status, stderr } => write!(f, "initdb failed, {status}: {}", stderr.trim_end()),
            Error::Timeout => write!(f, "initdb did not finish in time"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Spawn(error) => Some(error),
            Error::EmptyPassword(_) | Error::Initdb { .. } | Error::Timeout => None,
        }
    }
}
