//! Why a container could not be started.

use std::fmt;

use super::NoProvider;
use super::fuse::MountsError;
use crate::daemons;
use crate::logs;
use crate::store;

/// What a start fails with, in the order a start goes: the records,
/// the template, the provider — or, a connected tool, the daemon
/// record and the connection to it — the mounts, the run itself, the
/// log.
pub enum StartError {
    /// The records could not be read or written.
    Store(store::Error),
    /// The template the record names is not on record any more.
    NoTemplate(String),
    /// No provider to run on.
    Provider(NoProvider),
    /// The daemon record a connected tool names is not on record any
    /// more.
    NoDaemon(String),
    /// No connection to the daemon named could be opened.
    Daemon(daemons::Fail),
    /// A mount could not be served: a volume the provider refused, or
    /// a provider not connected.
    Mounts(String),
    /// A provider's capabilities exclude what the run or a mount
    /// needs, and it was not asked: what it lacks, in a sentence.
    Unsupported(String),
    /// The provider did not run it, in its own words or the wire's.
    Run(String),
    /// The log could not be written.
    Logs(logs::Error),
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartError::Store(error) => write!(f, "{error}"),
            StartError::NoTemplate(id) => write!(f, "the template {id} is not on record"),
            StartError::Provider(error) => write!(f, "{error}"),
            StartError::NoDaemon(name) => write!(f, "the daemon {name} is not on record"),
            StartError::Daemon(error) => write!(f, "{error}"),
            StartError::Mounts(error) => write!(f, "a mount could not be served: {error}"),
            StartError::Unsupported(what) => write!(f, "a provider's capabilities exclude it: {what}"),
            StartError::Run(error) => write!(f, "the provider did not run the container: {error}"),
            StartError::Logs(error) => write!(f, "the log: {error}"),
        }
    }
}

impl fmt::Debug for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl StartError {
    /// What a provider's capabilities lacked, where that is why the
    /// start failed: the sentence the daemon's `unsupported` error
    /// carries, in place of a failure.
    pub fn unsupported(&self) -> Option<&str> {
        match self {
            StartError::Unsupported(what) => Some(what),
            StartError::Provider(error) => error.unsupported(),
            _ => None,
        }
    }

    /// A run's mounts that could not be built, as the start's error.
    pub fn mounts(error: MountsError) -> StartError {
        match error {
            MountsError::Unsupported(what) => StartError::Unsupported(what),
            MountsError::Failed(what) => StartError::Mounts(what),
        }
    }
}

impl From<store::Error> for StartError {
    fn from(error: store::Error) -> Self {
        StartError::Store(error)
    }
}

impl From<logs::Error> for StartError {
    fn from(error: logs::Error) -> Self {
        StartError::Logs(error)
    }
}
