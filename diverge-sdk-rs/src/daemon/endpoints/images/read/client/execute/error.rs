//! What the daemon's read scope could not do before it was answered
//! held.

use std::fmt;

use super::super::super::server::response;
use crate::shared::error::Error;
use crate::wire::client::handle::SendError;
use crate::wire::frame;

/// A read scope that did not come to be held. Everything after the held
/// is each ask's to report.
///
/// Six of these are this end's view of something going wrong. The last
/// three are the daemon speaking: a not found and a forbidden are
/// answers, and an error is the daemon saying it could not answer.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out. See [`SendError`].
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it: a request the daemon
    /// could not read.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// No source the daemon asked can supply the image, and it said so;
    /// its finish follows, and no ask is answered.
    NotFound,
    /// The account the request is served for holds `oci` over neither
    /// providers nor daemons; nothing was asked.
    Forbidden,
    /// The daemon could not answer, and said so.
    ///
    /// **Not a not found.** This is the absence of an answer, where
    /// [`NotFound`](Self::NotFound) is one.
    Daemon(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "images read request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "images read request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the images read scope answered"),
            ExecuteError::Frame(error) => write!(f, "images read answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("images read scope finished without an answer"),
            ExecuteError::Response(error) => write!(f, "images read answer did not parse: {error}"),
            ExecuteError::NotFound => f.write_str("the daemon cannot supply the image"),
            ExecuteError::Forbidden => f.write_str("images read forbidden"),
            ExecuteError::Daemon(_) => f.write_str("the daemon could not answer the images read scope"),
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Daemon`](ExecuteError::Daemon) has no source, because what it
    /// carries is not a Rust error — see
    /// [`shared::error::Error`](crate::shared::error::Error).
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::NotFound
            | ExecuteError::Forbidden
            | ExecuteError::Daemon(_) => None,
        }
    }
}
