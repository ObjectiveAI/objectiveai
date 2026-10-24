//! Why a token could not be signed.

use std::fmt;

/// What [`access`](super::access) and [`id`](super::id) fail with.
#[derive(Debug)]
pub enum Error {
    /// The access token could not be signed.
    Access(jsonwebtoken::errors::Error),
    /// The ID token could not be signed: why.
    Id(String),
    /// The issuer identifier is not a URL: why.
    Issuer(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Access(error) => write!(f, "the access token could not be signed: {error}"),
            Error::Id(why) => write!(f, "the ID token could not be signed: {why}"),
            Error::Issuer(why) => write!(f, "the issuer identifier is not a URL: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Access(error) => Some(error),
            Error::Id(_) | Error::Issuer(_) => None,
        }
    }
}
