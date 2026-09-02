//! A continuation fetch that cannot answer.

use std::error;
use std::fmt;

use crate::filesystem::continuation::{CheckError, IngestError};

/// A continuation fetch that cannot answer.
#[derive(Debug)]
pub enum ContinuationError {
    /// The ask channel is gone — the socket driver died, so no
    /// delivery can come and waiting would be forever.
    Closed,
    /// The server failed the delivery — the bytes can never come
    /// (the client vanished mid-fetch, refused, …) — and this is its
    /// full error, verbatim off the error route.
    Failed(serde_json::Value),
    /// The bytes came but could not be landed.
    Ingest(IngestError),
    /// The bytes landed but the database did not check out.
    Check(CheckError),
    /// The continuation was already collected. One run, one fetch.
    Taken,
}

impl fmt::Display for ContinuationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContinuationError::Closed => {
                f.write_str("the ask channel is closed")
            }
            ContinuationError::Failed(error) => {
                write!(f, "the server failed the continuation: {error}")
            }
            ContinuationError::Ingest(error) => {
                write!(f, "the continuation could not be landed: {error}")
            }
            ContinuationError::Check(error) => write!(f, "{error}"),
            ContinuationError::Taken => {
                f.write_str("the continuation was already collected")
            }
        }
    }
}

impl error::Error for ContinuationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ContinuationError::Ingest(error) => Some(error),
            ContinuationError::Check(error) => Some(error),
            ContinuationError::Closed
            | ContinuationError::Failed(_)
            | ContinuationError::Taken => None,
        }
    }
}

impl From<CheckError> for ContinuationError {
    fn from(error: CheckError) -> Self {
        ContinuationError::Check(error)
    }
}
