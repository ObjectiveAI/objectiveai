//! Why a `_meta` is not what the daemon attests.

use std::fmt;

use crate::shared::mcp::KEY;

/// A `_meta` read as what the daemon attests, and found otherwise.
#[derive(Debug)]
pub enum Error {
    /// [`KEY`] is not there.
    Missing,
    /// [`KEY`] is there, and its value is not the attestation the
    /// reader is for: the other side's kind, or not the type at all.
    Invalid(serde_json::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing => write!(f, "`_meta` has no `{KEY}`"),
            Error::Invalid(source) => write!(
                f,
                "`_meta`'s `{KEY}` is not what this side is attested: {source}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Missing => None,
            Error::Invalid(source) => Some(source),
        }
    }
}
