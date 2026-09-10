//! A client-opened channel that never opened.

use std::fmt;

use crate::client::handle::SendError;

/// A channel request that did not go out.
///
/// Two ways, and both are before the wire: the frame would not
/// serialize, or the write failed. Everything a provider might object
/// to is objected to afterwards, on the channel, and the channel's
/// own reader reports it.
#[derive(Debug)]
pub enum OpenError {
    /// The channel request would not serialize.
    Request(serde_json::Error),
    /// The request never went out. See [`SendError`] for the three
    /// reasons, only one of which is about this scope rather than the
    /// whole connection.
    Send(SendError),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::Request(error) => {
                write!(f, "channel request did not serialize: {error}")
            }
            OpenError::Send(error) => write!(f, "the channel request never went out: {error}"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OpenError::Request(error) => Some(error),
            OpenError::Send(error) => Some(error),
        }
    }
}
