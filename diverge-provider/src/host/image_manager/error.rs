//! Why the store did not answer.

use std::fmt;

use diverge_sdk::shared::error;
use serde_json::json;

/// What [`ImageManager`](super::ImageManager) fails with: this
/// provider keeps no client images yet.
#[derive(Debug)]
pub enum Error {
    /// The store does not exist on this provider.
    Unsupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unsupported => f.write_str("this provider keeps no client images yet"),
        }
    }
}

impl std::error::Error for Error {}

/// What the protocol puts on the wire for one of these.
///
/// ```json
/// {"kind":"unsupported","error":"this provider keeps no client images yet"}
/// ```
impl From<Error> for error::Error {
    fn from(error: Error) -> Self {
        let kind = match &error {
            Error::Unsupported => "unsupported",
        };
        error::Error(json!({
            "kind": kind,
            "error": error.to_string(),
        }))
    }
}
