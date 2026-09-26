//! Why the registry could not start, or could not serve.

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
    /// A repository by this name is being served already. The SDK
    /// mints a fresh name per run, so this is a run handed the same
    /// name twice.
    Taken(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Bind(error) => write!(f, "the registry could not listen: {error}"),
            Error::Taken(repository) => write!(f, "the repository `{repository}` is served already"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Bind(error) => Some(error),
            Error::Taken(_) => None,
        }
    }
}

/// What the SDK puts on the wire for one of these.
///
/// ```json
/// {"kind":"taken","error":"the repository `…` is served already"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Bind(_) => "bind",
            Error::Taken(_) => "taken",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
