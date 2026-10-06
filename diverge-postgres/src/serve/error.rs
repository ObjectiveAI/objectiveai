//! Why the program could not start, or ended other than by being told.

use std::fmt;
use std::io;
use std::process::ExitStatus;

use diverge_sdk::file_lock;

use crate::cluster;
use crate::config;
use crate::install;
use crate::postmaster;

/// What [`run`](super::run) fails with. A `main` returning one of
/// these prints it, and that is the one report there is, so its
/// `Debug` is its `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The directory or the file could not be used.
    Config(config::Error),
    /// The binaries could not be had.
    Install(install::Error),
    /// The init lock could not be taken.
    Lock(file_lock::Error),
    /// The password could not be had, or the cluster could not be
    /// made.
    Cluster(cluster::Error),
    /// The postmaster could not be stopped, started, or reached.
    Postmaster(postmaster::Error),
    /// The ready line could not be written as JSON.
    Announce(serde_json::Error),
    /// The ready line could not be written to stdout.
    Stdout(io::Error),
    /// The postmaster could not be waited on.
    Wait(io::Error),
    /// The postmaster exited on its own, and this is how.
    Exited(ExitStatus),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Runtime(error) => write!(f, "the runtime could not be built: {error}"),
            Error::Config(error) => write!(f, "the configuration could not be used: {error}"),
            Error::Install(error) => write!(f, "the binaries could not be had: {error}"),
            Error::Lock(error) => write!(f, "the init lock could not be taken: {error}"),
            Error::Cluster(error) => write!(f, "the cluster could not be made ready: {error}"),
            Error::Postmaster(error) => write!(f, "the postmaster: {error}"),
            Error::Announce(error) => write!(f, "the ready line could not be written: {error}"),
            Error::Stdout(error) => write!(f, "stdout could not be written: {error}"),
            Error::Wait(error) => write!(f, "the postmaster could not be waited on: {error}"),
            Error::Exited(status) => write!(f, "the postmaster exited on its own: {status}"),
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
            Error::Runtime(error) | Error::Stdout(error) | Error::Wait(error) => Some(error),
            Error::Config(error) => Some(error),
            Error::Install(error) => Some(error),
            Error::Lock(error) => Some(error),
            Error::Cluster(error) => Some(error),
            Error::Postmaster(error) => Some(error),
            Error::Announce(error) => Some(error),
            Error::Exited(_) => None,
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}

impl From<install::Error> for Error {
    fn from(error: install::Error) -> Self {
        Error::Install(error)
    }
}

impl From<cluster::Error> for Error {
    fn from(error: cluster::Error) -> Self {
        Error::Cluster(error)
    }
}

impl From<postmaster::Error> for Error {
    fn from(error: postmaster::Error) -> Self {
        Error::Postmaster(error)
    }
}
