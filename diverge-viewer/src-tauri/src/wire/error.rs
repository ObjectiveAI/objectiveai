//! Why the daemon could not be dialled.

use std::fmt;

/// What [`dial`](fn@super::dial) fails with: no socket, or a handshake
/// that did not complete.
#[derive(Debug)]
pub enum Error {
    /// The socket could not be made: the address did not resolve,
    /// nothing listens there, or the WebSocket handshake was refused.
    Dial(String),
    /// The socket was made and the credential could not be presented
    /// on it.
    Authorize(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Dial(why) => write!(f, "the daemon could not be dialled: {why}"),
            Error::Authorize(why) => write!(f, "the credential could not be presented: {why}"),
        }
    }
}

impl std::error::Error for Error {}
