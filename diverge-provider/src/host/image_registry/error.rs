//! Why the registry could not start, or could not mint.

use std::fmt;
use std::io;

use diverge_sdk::shared::error;
use serde_json::json;

/// What [`ImageRegistry`](super::ImageRegistry) fails with.
#[derive(Debug)]
pub enum Error {
    /// The loopback listener could not be bound, so there is no
    /// registry.
    Bind(io::Error),
    /// The operating system's random source did not answer, so no
    /// token could be minted.
    Random(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Bind(error) => write!(f, "the registry could not listen: {error}"),
            Error::Random(error) => write!(f, "no token could be minted: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Bind(error) => Some(error),
            Error::Random(_) => None,
        }
    }
}

/// What the protocol puts on the wire for one of these.
///
/// ```json
/// {"kind":"random","error":"no token could be minted: …"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Bind(_) => "bind",
            Error::Random(_) => "random",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
