//! Why the issuer could not start, or could not listen.

use std::fmt;
use std::io;
use std::path::PathBuf;

use diverge_sdk::file_lock;

use crate::config;
use crate::store;

/// What [`run`](super::run) fails with. A `main` returning one of
/// these prints it, and that is the one report a failed start gets,
/// so its `Debug` is its `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The root or the file could not be used.
    Config(config::Error),
    /// The root's lock file could not be taken, written or read.
    Lock(file_lock::Error),
    /// Another issuer holds the root: the root, and the process id
    /// the holder left in the lock file, where it left one.
    Locked {
        /// The root held.
        root: PathBuf,
        /// The holder's process id, as it wrote it.
        holder: Option<String>,
    },
    /// `issuer.sqlite` could not be opened.
    Store(store::Error),
    /// The port could not be bound.
    Bind(io::Error),
    /// The listener stopped on its own.
    Serve(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(error) => write!(f, "the runtime could not be built: {error}"),
            Error::Config(error) => write!(f, "the configuration could not be used: {error}"),
            Error::Lock(error) => write!(f, "the root could not be locked: {error}"),
            Error::Locked { root, holder } => match holder {
                Some(holder) => write!(f, "another issuer holds {}: process {holder}", root.display()),
                None => write!(f, "another issuer holds {}", root.display()),
            },
            Error::Store(error) => write!(f, "{error}"),
            Error::Bind(error) => write!(f, "the port could not be bound: {error}"),
            Error::Serve(error) => write!(f, "the listener stopped: {error}"),
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
            Error::Runtime(error) | Error::Bind(error) | Error::Serve(error) => Some(error),
            Error::Config(error) => Some(error),
            Error::Lock(error) => Some(error),
            Error::Locked { .. } => None,
            Error::Store(error) => Some(error),
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}
