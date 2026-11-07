//! What a send could not do before its scope was open.

use std::fmt;

use crate::wire::client::handle::SendError;

/// A send that could not be opened. Everything after the opening
/// is the scope's to report.
#[derive(Debug)]
pub enum ExecuteError {
    /// The receiver's identity did not serialize, and nothing was sent.
    Request(postcard::Error),
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Request(error) => write!(f, "send request did not serialize: {error}"),
            ExecuteError::Send(error) => write!(f, "send request could not be sent: {error}"),
        }
    }
}

impl std::error::Error for ExecuteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Send(error) => Some(error),
        }
    }
}
