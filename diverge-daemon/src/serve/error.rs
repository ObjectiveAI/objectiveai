//! Why the daemon could not start, or could not listen.

use std::fmt;
use std::io;

use crate::config;
use crate::database;
use crate::postgres;
use crate::store;

/// What [`run`](super::run) fails with: the configuration, the
/// database, the store, or the port. A `main` returning one of these
/// prints it, and that is the one report a failed start gets, so its
/// `Debug` is its `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The directory or the file could not be used.
    Config(config::Error),
    /// The local Postgres could not be started.
    Postgres(postgres::Error),
    /// The store could not be opened, or its schema applied.
    Store(store::Error),
    /// The database URL could not be read for where and how to dial.
    Database(database::TargetError),
    /// The content directory or the logs directory could not be made.
    Resources(io::Error),
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
            Error::Postgres(error) => write!(f, "the local Postgres could not be started: {error}"),
            Error::Store(error) => write!(f, "the store could not be opened: {error}"),
            Error::Database(error) => write!(f, "{error}"),
            Error::Resources(error) => write!(f, "the resources or agents directory could not be made: {error}"),
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
            Error::Runtime(error) | Error::Bind(error) | Error::Serve(error) | Error::Resources(error) => Some(error),
            Error::Config(error) => Some(error),
            Error::Postgres(error) => Some(error),
            Error::Store(error) => Some(error),
            Error::Database(error) => Some(error),
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}
