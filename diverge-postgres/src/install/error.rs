//! Why the binaries could not be had.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_sdk::file_lock;

/// What [`ensure`](super::ensure) fails with.
#[derive(Debug)]
pub enum Error {
    /// A directory or a file under `bin/` could not be made, renamed
    /// or written.
    Io {
        /// What could not be.
        path: PathBuf,
        /// Why.
        source: io::Error,
    },
    /// The install lock could not be taken.
    Lock(file_lock::Error),
    /// The archive could not be extracted, or the `initdb` that proves
    /// an extraction failed, every time it was tried — or the
    /// extractor that makes its hard links could not be registered.
    Extract(postgresql_embedded::Error),
    /// The archive was extracted to this directory, and what is there
    /// is without the hard links the archive names all the same: see
    /// [`linked`](super::linked).
    Links(PathBuf),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "`{}` could not be used: {source}", path.display()),
            Error::Lock(error) => write!(f, "the install lock could not be taken: {error}"),
            Error::Extract(error) => write!(f, "the Postgres binaries could not be extracted: {error}"),
            Error::Links(home) => write!(f, "`{}` was extracted without the archive's hard links: its `share/timezone/UTC` is missing", home.display()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Lock(error) => Some(error),
            Error::Extract(error) => Some(error),
            Error::Links(_) => None,
        }
    }
}
