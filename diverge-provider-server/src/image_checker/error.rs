//! Why a check did not answer.

use std::fmt;

use diverge_provider_sdk::shared::error;
use serde_json::json;

use crate::tools;

/// What [`ImageChecker`](super::ImageChecker) fails with: podman
/// could not be run, or answered with an exit that is neither yes
/// nor no. A failure to answer, never a negative answer.
#[derive(Debug)]
pub enum Error {
    /// Podman did not answer.
    Podman(tools::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Podman(error) => write!(f, "podman did not answer: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Podman(error) => Some(error),
        }
    }
}

impl From<tools::Error> for Error {
    fn from(error: tools::Error) -> Self {
        Error::Podman(error)
    }
}

/// What the SDK puts on the wire for one of these.
///
/// ```json
/// {"kind":"podman","error":"podman did not answer: …"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Podman(_) => "podman",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
