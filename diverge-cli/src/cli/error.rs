//! Why the program could not run.

use std::fmt;
use std::io;

use diverge_sdk::identities;

use crate::config;
use crate::document;
use crate::lifecycle;
use crate::run;

/// What [`run`](super::run) fails with: the runtime, the
/// configuration, a document, a request, the identities, or a
/// program's start or stop. A `main` returning one of these prints it, and that
/// is the one report a failed run gets, so its `Debug` is its
/// `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The root, the file, or the names could not be used.
    Config(config::Error),
    /// A document could not be read, rewritten or edited.
    Document(document::Error),
    /// A request could not be put, or its answers printed.
    Run(run::Error),
    /// The daemon or the provider could not be started or stopped.
    Lifecycle(lifecycle::Error),
    /// The root's identities refused, could not be opened, or a login
    /// did not come to an identity.
    Identities(identities::Error),
    /// Stdout could not be written.
    Stdout(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(error) => write!(f, "the runtime could not be built: {error}"),
            Error::Config(error) => write!(f, "{error}"),
            Error::Document(error) => write!(f, "{error}"),
            Error::Run(error) => write!(f, "{error}"),
            Error::Lifecycle(error) => write!(f, "{error}"),
            Error::Identities(error) => write!(f, "{error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
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
            Error::Document(error) => Some(error),
            Error::Run(error) => Some(error),
            Error::Lifecycle(error) => Some(error),
            Error::Identities(error) => Some(error),
            Error::Stdout(error) => Some(error),
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}

impl From<document::Error> for Error {
    fn from(error: document::Error) -> Self {
        Error::Document(error)
    }
}

impl From<run::Error> for Error {
    fn from(error: run::Error) -> Self {
        Error::Run(error)
    }
}

impl From<identities::Error> for Error {
    fn from(error: identities::Error) -> Self {
        Error::Identities(error)
    }
}

impl From<lifecycle::Error> for Error {
    fn from(error: lifecycle::Error) -> Self {
        Error::Lifecycle(error)
    }
}
