//! Why a container could not be started.

use std::fmt;

use super::NoProvider;
use crate::logs;
use crate::store;

/// What a start fails with, in the order a start goes: the records,
/// the template, the provider, the mounts, the run itself, the log.
pub enum StartError {
    /// The records could not be read or written.
    Store(store::Error),
    /// The template the record names is not on record any more.
    NoTemplate(String),
    /// No provider to run on.
    Provider(NoProvider),
    /// A mount could not be served: a volume the provider refused, or
    /// a provider not connected.
    Mounts(String),
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
            StartError::Mounts(error) => write!(f, "a mount could not be served: {error}"),
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
