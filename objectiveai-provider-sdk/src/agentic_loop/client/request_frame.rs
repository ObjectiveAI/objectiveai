//! What a client's request frame carries.

use super::request::AgenticLoopRequest;

/// The payload of a [`ClientFrame::Request`](crate::frame::client::ClientFrame::Request).
///
/// A client makes exactly one request, and everything else in the
/// connection hangs off it: the scope the server mints to answer it,
/// the channels the server opens inside that scope, and the chunks
/// that come back on channel `0`.
///
/// One variant, and an enum anyway — but for a different reason than
/// [`ServerBodyFrame`](super::super::server::ServerBodyFrame)'s. The
/// client's frame types are a CLOSED set, so this will not grow by a
/// new type byte. It is an enum because the alternative is a newtype
/// that reads like a synonym, and because a second kind of client
/// request would be a protocol change worth seeing as one.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientRequestFrame {
    /// Run an agent, and stream back what it does.
    AgenticLoop(AgenticLoopRequest),
}
