//! What a client's body frame carries.

use super::McpResponseHead;

/// The payload of a [`ClientFrame::Body`](crate::frame::client::ClientFrame::Body).
///
/// What came back on a channel the SERVER opened. A client never
/// streams anything of its own — its own request is a frame entire,
/// and channel `0` belongs to the server — so every body frame it
/// sends is an answer to something it was asked for.
///
/// # How a reader tells these apart
///
/// Not from the bytes: a client body frame has no type byte, and needs
/// none. The channel's kind was settled by the
/// [`ServerRequestFrame`](super::super::server::ServerRequestFrame)
/// that opened it, so a reader already knows whether it is receiving
/// MCP or Postgres before the first body arrives.
///
/// Within an MCP channel the two variants are told apart by POSITION.
/// The first body frame is the head; every one after it is body bytes.
/// That ordering is the whole encoding — there is no length, no count,
/// and no terminator beyond the channel's finish.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientBodyFrame<'a> {
    /// The status and headers of an MCP response. Always the first
    /// body frame on an MCP channel, and never repeated.
    McpHead(McpResponseHead),
    /// A piece of an MCP response body: the whole thing for a single
    /// JSON answer, or one event's worth for a stream. Every body
    /// frame on an MCP channel after the head is one of these.
    McpBody(&'a [u8]),
    /// Postgres bytes, from the database. Opaque, and a stream, for
    /// the reasons given on
    /// [`ServerRequestFrame`](super::super::server::ServerRequestFrame).
    Postgres(&'a [u8]),
}
