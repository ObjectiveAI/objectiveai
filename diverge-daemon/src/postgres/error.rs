//! Why the local Postgres could not be started.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// What [`start`](super::start) fails with in the local mode. The
/// remote mode fails nowhere here: a URL that cannot be reached is
/// the store's error.
#[derive(Debug)]
pub enum Error {
    /// Where the daemon's own executable is could not be learned, so
    /// the program beside it could not be found.
    Executable(io::Error),
    /// The program could not be started.
    Spawn {
        /// The program, where it was looked for.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// Its stdout could not be read.
    Stdout(io::Error),
    /// Its stdout ended before it wrote the ready line: the program
    /// failed to start, and said why on its own stderr.
    Ended,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Executable(error) => write!(f, "the daemon's own executable could not be located: {error}"),
            Error::Spawn { path, source } => write!(f, "`{}` could not be started: {source}", path.display()),
            Error::Stdout(error) => write!(f, "diverge-postgres could not be read: {error}"),
            Error::Ended => write!(f, "diverge-postgres ended before it was ready"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Executable(error) | Error::Stdout(error) => Some(error),
            Error::Spawn { source, .. } => Some(source),
            Error::Ended => None,
        }
    }
}
