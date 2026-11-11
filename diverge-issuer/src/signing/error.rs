//! Why the signing keys could not be read or made.

use std::fmt;


use crate::store;

/// What [`Signing::load`](super::Signing::load), [`mint`](super::mint) and
/// [`rotate`](super::rotate) fail with.
#[derive(Debug)]
pub enum Error {
    /// The database refused.
    Store(store::Error),
    /// A key could not be drawn.
    Generate(rsa::Error),
    /// A key's PEM could not be read or written: the key id, and why.
    Pem(String, String),
    /// A key could not be made ready for the access token.
    Jwt(jsonwebtoken::errors::Error),
    /// The blocking task a key was drawn on panicked.
    Panicked,
    /// The key set could not be written as JSON.
    KeySet(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Store(error) => write!(f, "{error}"),
            Error::Generate(error) => write!(f, "a signing key could not be drawn: {error}"),
            Error::Pem(kid, why) => write!(f, "the signing key `{kid}` is not a PKCS#1 RSA key: {why}"),
            Error::Jwt(error) => write!(f, "a signing key could not be read: {error}"),
            Error::Panicked => f.write_str("the drawing of a signing key panicked"),
            Error::KeySet(error) => write!(f, "the key set could not be written: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Store(error) => Some(error),
            Error::Generate(error) => Some(error),
            Error::Jwt(error) => Some(error),
            Error::KeySet(error) => Some(error),
            Error::Pem(..) | Error::Panicked => None,
        }
    }
}
