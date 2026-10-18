//! Why a check did not answer.

use std::fmt;

use diverge_sdk::shared::error;
use serde_json::json;

use crate::host::tools;

/// What [`ImageChecker`](super::ImageChecker) fails with: a reference
/// whose name could not be put into a registry reference, or podman
/// not starting to ask a registry. A failure to answer, never a
/// negative answer.
#[derive(Debug)]
pub enum Error {
    /// A reference's name is not a repository path: refused, never
    /// normalized, since it lands in a reference by concatenation.
    Name(String),
    /// Podman did not answer.
    Podman(tools::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Name(name) => write!(f, "the image name `{name}` is not a repository path"),
            Error::Podman(error) => write!(f, "podman did not answer: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Name(_) => None,
            Error::Podman(error) => Some(error),
        }
    }
}

/// What the protocol puts on the wire for one of these.
///
/// ```json
/// {"kind":"podman","error":"podman did not answer: …"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Name(_) => "name",
            Error::Podman(_) => "podman",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
