//! A preparation that could not be completed.

use std::error;
use std::fmt;
use std::io;

use crate::continuation_fetcher;
use crate::resource_fetcher::FetchError;

/// Why the filesystem could not be prepared. No variant carries a
/// secret: a contradiction names the variable, not its values.
#[derive(Debug)]
pub enum PrepareError {
    /// The request's agent is not a `hermes` one.
    WrongAgent,
    /// The request sets one environment variable to two different
    /// values — the same credential supplied twice, disagreeing.
    Contradiction {
        /// The variable both wanted.
        variable: &'static str,
    },
    /// A resource could not be fetched.
    Resource {
        /// The request field that named it.
        field: &'static str,
        /// Why.
        error: FetchError,
    },
    /// A resource was fetched but is not a JSON object, which every
    /// state document here must be.
    ResourceNotObject {
        /// The request field that named it.
        field: &'static str,
    },
    /// The continuation could not be had, or did not check out.
    Continuation(continuation_fetcher::FetchError),
    /// A file or directory could not be written.
    Io(io::Error),
    /// A document would not serialize — which plain data never
    /// fails to do, so this names a bug rather than a circumstance.
    Json(serde_json::Error),
}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrepareError::WrongAgent => {
                f.write_str("the request's agent is not a hermes agent")
            }
            PrepareError::Contradiction { variable } => write!(
                f,
                "the request sets {variable} to two different values"
            ),
            PrepareError::Resource { field, error } => {
                write!(f, "the resource {field} could not be fetched: {error}")
            }
            PrepareError::ResourceNotObject { field } => {
                write!(f, "the resource {field} is not a JSON object")
            }
            PrepareError::Continuation(error) => write!(f, "{error}"),
            PrepareError::Io(error) => {
                write!(f, "the filesystem could not be prepared: {error}")
            }
            PrepareError::Json(error) => {
                write!(f, "a document would not serialize: {error}")
            }
        }
    }
}

impl error::Error for PrepareError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            PrepareError::Resource { error, .. } => Some(error),
            PrepareError::Continuation(error) => Some(error),
            PrepareError::Io(error) => Some(error),
            PrepareError::Json(error) => Some(error),
            PrepareError::WrongAgent
            | PrepareError::Contradiction { .. }
            | PrepareError::ResourceNotObject { .. } => None,
        }
    }
}

impl From<io::Error> for PrepareError {
    fn from(error: io::Error) -> Self {
        PrepareError::Io(error)
    }
}

impl From<serde_json::Error> for PrepareError {
    fn from(error: serde_json::Error) -> Self {
        PrepareError::Json(error)
    }
}
