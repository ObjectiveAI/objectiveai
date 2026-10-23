//! Why the keys could not be read.

use std::fmt;

use diverge_sdk::keyring;

/// What [`Keys::resolve`](super::Keys::resolve) fails with.
#[derive(Debug)]
pub enum Error {
    /// The keyring could not be read.
    Keyring(keyring::Error),
    /// The keyring holds no entry of the name the document names.
    Missing(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Keyring(error) => write!(f, "the keyring could not be read: {error}"),
            Error::Missing(entry) => write!(f, "the keyring holds no `{entry}`, which provider.yaml names"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Keyring(error) => Some(error),
            Error::Missing(_) => None,
        }
    }
}
