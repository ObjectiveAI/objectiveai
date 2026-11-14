//! What a read scope could not do before it was answered held.

use std::fmt;

use super::super::super::server::response;
use crate::shared::error::Error;
use crate::wire::client::handle::SendError;
use crate::wire::frame;

/// A read scope that did not come to be held. Everything after the
/// held is each ask's to report.
///
/// Six of these are this end's view of something going wrong. The
/// last three are the provider speaking: a not found and an
/// unsupported are answers, and an error is the provider saying it
/// could not answer.
#[derive(Debug)]
pub enum ExecuteError {
    /// The request never went out.
    ///
    /// Nothing was written, so nothing is waiting to answer it. See
    /// [`SendError`] for the three reasons, only one of which is about
    /// this exchange rather than the whole connection.
    Send(SendError),
    /// The request would not serialize.
    Request(serde_json::Error),
    /// The connection ended before anything came back.
    ///
    /// Which is the only way waiting stops early. Nothing times a
    /// provider out.
    Closed,
    /// What came back was not a frame.
    Frame(frame::FrameError),
    /// The scope finished without an answer in it.
    ///
    /// A provider sends exactly one response first on a read scope. One
    /// that sends only the finish has said nothing at all, which is
    /// different from saying no.
    Unanswered,
    /// The response frame did not parse.
    Response(response::FrameError),
    /// The provider cannot supply the image — neither the caller's
    /// Store nor a registry it is configured with, named by one of the
    /// image's references, has it — and said so; its finish follows,
    /// and no ask is answered.
    NotFound,
    /// The provider has neither a Store nor any registry, and answered
    /// so: it has nowhere to look, and nothing is determined about the
    /// image. Ask another provider.
    Unsupported,
    /// The provider could not answer, and said so.
    ///
    /// **Not a not found.** This is the absence of an answer, where
    /// [`NotFound`](Self::NotFound) is one.
    Provider(Error),
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecuteError::Send(error) => write!(f, "image read request could not be sent: {error}"),
            ExecuteError::Request(error) => write!(f, "image read request did not serialize: {error}"),
            ExecuteError::Closed => f.write_str("connection ended before the image read scope answered"),
            ExecuteError::Frame(error) => write!(f, "image read answer did not decode: {error}"),
            ExecuteError::Unanswered => f.write_str("image read scope finished without an answer"),
            ExecuteError::Response(error) => write!(f, "image read answer did not parse: {error}"),
            ExecuteError::NotFound => f.write_str("the provider cannot supply the image"),
            ExecuteError::Unsupported => f.write_str("the provider has neither a Store nor any registry"),
            ExecuteError::Provider(_) => f.write_str("the provider could not answer the image read scope"),
        }
    }
}

impl std::error::Error for ExecuteError {
    /// [`Provider`](ExecuteError::Provider) has no source, because what
    /// it carries is not a Rust error and deliberately does not
    /// implement one — see
    /// [`shared::error::Error`](crate::shared::error::Error). A caller
    /// that wants what is inside it matches the variant.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecuteError::Send(error) => Some(error),
            ExecuteError::Request(error) => Some(error),
            ExecuteError::Frame(error) => Some(error),
            ExecuteError::Response(error) => Some(error),
            ExecuteError::Closed
            | ExecuteError::Unanswered
            | ExecuteError::NotFound
            | ExecuteError::Unsupported
            | ExecuteError::Provider(_) => None,
        }
    }
}
