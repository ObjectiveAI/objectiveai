//! One piece of an MCP answer, owned.

use bytes::Bytes;

use crate::shared::http::response;

/// The head of an MCP answer, or a piece of its body.
///
/// What an [`McpStream`](super::McpStream) yields. The same two things
/// [`mcp::Frame`](super::super::super::server::channel_response::mcp::Frame)
/// carries on the wire, owning its body instead of borrowing it —
/// which is the whole reason this exists. A frame decoded out of a
/// message borrows that message, and a stream item outlives the poll
/// that produced it.
///
/// Owning is not copying. The body is a refcounted view of the frame it
/// arrived in, so keeping one keeps that frame alive and nothing is
/// duplicated.
///
/// # The order is the protocol's, and it is worth relying on
///
/// A [`Head`](Self::Head) comes once, first, and never again. Every
/// item after it is a [`Body`](Self::Body). That is not a convention
/// this type enforces — it is what the far end sends, and an
/// [`McpStream`](super::McpStream) reports a second head as an error
/// rather than passing it on.
///
/// So a caller may take the first item as the head and treat the rest
/// as body without checking, and a caller that would rather match on
/// every item gets the same answer.
#[derive(Debug, Clone, PartialEq)]
pub enum McpFrame {
    /// The status and headers, once.
    ///
    /// Where `Mcp-Session-Id` arrives, which is how a connector learns
    /// its session id — the initialize response mints it, and nothing
    /// between the two ends reads it.
    ///
    /// Also where `Content-Type` says what the body is going to be: one
    /// JSON document, or an event stream held open for the session. The
    /// two are the same sequence of items here and differ only in how
    /// many bodies there are and how far apart they land.
    Head(response::Head),
    /// A piece of the body.
    ///
    /// The whole of it for a single JSON answer, one event's worth for
    /// a stream. How it is divided is the far server's business and
    /// carries no meaning: a caller reassembling a document
    /// concatenates, and a caller reading events parses the stream it
    /// was promised.
    Body(Bytes),
}
