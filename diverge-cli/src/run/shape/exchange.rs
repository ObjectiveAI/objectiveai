//! The executors' failures, told apart.

use diverge_sdk::daemon::client::one_shot;
use diverge_sdk::daemon::client::stream::{OpenError, StreamError};
use diverge_sdk::daemon::client::upload::UploadError;
use std::fmt::Display;

use crate::run::Error;

/// The one failure that is told apart: the connection closing with
/// no answer, which is how the daemon refuses a credential —
/// [`Error::Rejected`] — and every other as the executor's own words.
pub fn one_shot<Q: Display, A: Display>(error: one_shot::Error<Q, A>) -> Error {
    match error {
        one_shot::Error::Closed | one_shot::Error::Send(_) => Error::Rejected,
        other => Error::Exchange(other.to_string()),
    }
}

/// Likewise, for a scope that could not be opened.
pub fn open<Q: Display>(error: OpenError<Q>) -> Error {
    match error {
        OpenError::Send(_) => Error::Rejected,
        other => Error::Exchange(other.to_string()),
    }
}

/// Likewise, for a stream that failed.
pub fn stream<E: Display>(error: StreamError<E>) -> Error {
    match error {
        StreamError::Closed => Error::Rejected,
        other => Error::Exchange(other.to_string()),
    }
}

/// Likewise, for an upload.
pub fn upload<Q: Display, A: Display>(error: UploadError<Q, A>) -> Error {
    match error {
        UploadError::Closed | UploadError::Send(_) => Error::Rejected,
        other => Error::Exchange(other.to_string()),
    }
}
