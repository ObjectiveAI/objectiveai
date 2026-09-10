//! The trait.

use std::fmt;

use bytes::Bytes;

/// How one kind of client-opened channel is read: what a frame on it
/// decodes to, and what ends it.
///
/// A marker type per answer, never constructed, so that
/// [`ChannelStream`](super::super::ChannelStream) and
/// [`unary`](super::super::unary) can be written once and read every
/// channel. [`decode`](Self::decode) takes the whole payload — the
/// bytes after the frame header — and answers the item, the
/// provider's refusal (which ends the channel from this end's point
/// of view), or a payload that would not parse.
pub trait Answered: fmt::Debug + Send + 'static {
    /// What a frame carries when the exchange is going well.
    type Item: Send + 'static;
    /// Why a payload could not be read.
    type Error: std::error::Error + Send + Sync + 'static;
    /// What the provider says when it will not, or cannot, go on: this
    /// crate's [`Error`](crate::shared::error::Error), an MCP
    /// [`ErrorData`](rmcp::model::ErrorData), or nothing at all for a
    /// channel that has no such frame.
    type Refusal: fmt::Debug + Send + 'static;

    /// Read one payload.
    fn decode(payload: Bytes) -> Result<Result<Self::Item, Self::Refusal>, Self::Error>;
}
