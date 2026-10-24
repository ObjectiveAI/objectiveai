//! Why a provider's connection did not come to be held.

use std::fmt;

use diverge_sdk::provider::endpoints::info::client::execute::ExecuteError;

/// What [`attach`](super::attach) fails with: the one thing asked of a
/// provider before it counts as connected.
#[derive(Debug)]
pub enum Error {
    /// The provider did not answer its info, or answered something
    /// that is not one: the socket is not a provider's, or is already
    /// gone.
    Info(ExecuteError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Info(error) => write!(f, "the provider did not answer its info: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Info(error) => Some(error),
        }
    }
}
