//! What a client's response frame carries.

use super::McpResponseHead;

/// The payload of a [`ClientFrame::Response`](crate::frame::client::ClientFrame::Response).
///
/// What came back on a channel the SERVER opened. A client never
/// streams anything of its own — its own request is a frame entire,
/// and channel `0` belongs to the server — so every response frame it
/// sends is an answer to something it was asked for.
///
/// # How a reader tells these apart
///
/// Not from the bytes: a client response frame has no type byte, and
/// needs none. The channel's kind was settled by the
/// [`ServerRequestFrame`](super::super::server::ServerRequestFrame)
/// that opened it, so a reader already knows whether it is receiving
/// MCP or Postgres before the first of these arrives.
///
/// Within an MCP channel the two variants are told apart by POSITION.
/// The first is the head; every one after it is body bytes. That
/// ordering is the whole encoding — there is no length, no count, and
/// no terminator beyond the channel's finish.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientResponseFrame<'a> {
    /// The status and headers of an MCP response. Always the first
    /// response frame on an MCP channel, and never repeated.
    McpHead(McpResponseHead),
    /// A piece of an MCP response body: the whole thing for a single
    /// JSON answer, or one event's worth for a stream. Every response
    /// frame on an MCP channel after the head is one of these.
    McpBody(&'a [u8]),
    /// Postgres bytes, from the database. Opaque, and a stream, for
    /// the reasons given on
    /// [`ServerRequestFrame`](super::super::server::ServerRequestFrame).
    Postgres(&'a [u8]),
}
