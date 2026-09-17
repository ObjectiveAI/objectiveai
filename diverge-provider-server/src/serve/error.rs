//! Why the provider could not start, or could not listen.

use std::fmt;
use std::io;

use crate::config;
use crate::container_deployer;
use crate::image_registry;

/// What [`run`](super::run) fails with: the configuration, a piece
/// that could not be made, or the port. A `main` returning one of
/// these prints it, and that is the one report a failed start gets,
/// so its `Debug` is its `Display`.
pub enum Error {
    /// The runtime could not be built.
    Runtime(io::Error),
    /// The directory or the file could not be used.
    Config(config::Error),
    /// The image registry could not start.
    Registry(image_registry::Error),
    /// The deployer could not be made: podman, its machine, the proxy
    /// binary, or a file under `run/`.
    Deployer(container_deployer::Error),
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
            Error::Registry(error) => write!(f, "the image registry could not start: {error}"),
            Error::Deployer(error) => write!(f, "the deployer could not be made: {error}"),
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
            Error::Runtime(error) => Some(error),
            Error::Config(error) => Some(error),
            Error::Registry(error) => Some(error),
            Error::Deployer(error) => Some(error),
            Error::Bind(error) => Some(error),
            Error::Serve(error) => Some(error),
        }
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Error::Config(error)
    }
}
