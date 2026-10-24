//! Why the program could not run.

use std::fmt;
use std::io;

use diverge_issuer::password;
use diverge_sdk::keyring;

use crate::config;
use crate::document;
use crate::lifecycle;
use crate::run;

/// What [`run`](super::run) fails with: the runtime, the
/// configuration, a document, a request, or a program's start or
/// stop. A `main` returning one of these prints it, and that
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
    /// The root's keyring refused, or could not be opened.
    Keyring(keyring::Error),
    /// Stdin could not be read.
    Stdin(io::Error),
    /// Stdout could not be written.
    Stdout(io::Error),
    /// The key on stdin was empty.
    EmptyKey,
    /// A link was not made whole: which step, and why.
    Link(String),
    /// The issuer's database refused, or could not be opened.
    Issuer(diverge_issuer::store::Error),
    /// A password could not be hashed.
    Password(password::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(error) => write!(f, "the runtime could not be built: {error}"),
            Error::Config(error) => write!(f, "{error}"),
            Error::Document(error) => write!(f, "{error}"),
            Error::Run(error) => write!(f, "{error}"),
            Error::Lifecycle(error) => write!(f, "{error}"),
            Error::Keyring(error) => write!(f, "{error}"),
            Error::Stdin(error) => write!(f, "stdin could not be read: {error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
            Error::EmptyKey => f.write_str("the key on stdin was empty"),
            Error::Link(why) => write!(f, "the link was not made: {why}"),
            Error::Issuer(error) => write!(f, "{error}"),
            Error::Password(error) => write!(f, "{error}"),
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
            Error::Keyring(error) => Some(error),
            Error::Stdin(error) | Error::Stdout(error) => Some(error),
            Error::EmptyKey | Error::Link(_) => None,
            Error::Issuer(error) => Some(error),
            Error::Password(error) => Some(error),
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

impl From<keyring::Error> for Error {
    fn from(error: keyring::Error) -> Self {
        Error::Keyring(error)
    }
}

impl From<diverge_issuer::store::Error> for Error {
    fn from(error: diverge_issuer::store::Error) -> Self {
        Error::Issuer(error)
    }
}

impl From<password::Error> for Error {
    fn from(error: password::Error) -> Self {
        Error::Password(error)
    }
}

impl From<lifecycle::Error> for Error {
    fn from(error: lifecycle::Error) -> Self {
        Error::Lifecycle(error)
    }
}
