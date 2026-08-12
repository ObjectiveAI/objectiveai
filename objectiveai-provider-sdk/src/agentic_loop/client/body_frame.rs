//! What a client's body frame carries.

/// The payload of a [`ClientFrame::Body`](crate::frame::client::ClientFrame::Body).
///
/// A body frame's bytes mean different things on different channels,
/// and this is the set of things they can mean. Which one applies is
/// not carried in the frame: it was settled when the channel opened,
/// by the type of the server request that opened it. A reader knows
/// before the body arrives, so the body does not repeat it.
///
/// Every variant carries its channel. A client only ever sends bodies
/// on channels the SERVER opened — its own request rides in a request
/// frame, not a body — so there is no channel `0` case to leave
/// implicit the way the server's has.
///
/// Both variants are opaque bytes, and stay separate because they are
/// different KINDS of channel rather than different formats. See
/// [`ServerBodyFrame`](super::super::server::ServerBodyFrame) for why
/// neither is parsed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientBodyFrame<'a> {
    /// MCP bytes, from the client's MCP server.
    Mcp {
        /// The channel the server opened for this exchange.
        channel: u32,
        /// The bytes, borrowed from the frame they arrived in.
        payload: &'a [u8],
    },
    /// Postgres bytes, from the database.
    Postgres {
        /// The channel the server opened for this connection.
        channel: u32,
        /// The bytes, borrowed from the frame they arrived in.
        payload: &'a [u8],
    },
}
