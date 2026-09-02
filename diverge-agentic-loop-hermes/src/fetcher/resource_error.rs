//! A resource fetch that cannot answer.

use std::error;
use std::fmt;
use std::string::FromUtf8Error;

/// A resource fetch that cannot answer.
#[derive(Debug)]
pub enum ResourceError {
    /// The ask channel is gone — the socket driver died, so no
    /// delivery can come and waiting would be forever.
    Closed,
    /// The server failed the delivery — the bytes can never come
    /// (the client disconnected, holds nothing, …) — and this is
    /// its full error, verbatim off the error route.
    Failed(serde_json::Value),
    /// The settled bytes are not UTF-8, which this container's
    /// resources must be.
    Utf8(FromUtf8Error),
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::Closed => {
                f.write_str("the ask channel is closed")
            }
            ResourceError::Failed(error) => {
                write!(f, "the server failed the resource: {error}")
            }
            ResourceError::Utf8(error) => {
                write!(f, "the resource is not UTF-8: {error}")
            }
        }
    }
}

impl error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ResourceError::Utf8(error) => Some(error),
            ResourceError::Closed | ResourceError::Failed(_) => None,
        }
    }
}
