//! What a client's response frame carries on an MCP channel.

/// One MCP answer: the head once, then as much body as there turns
/// out to be.
///
/// An alias, because an MCP channel carries a tunneled HTTP exchange
/// and the answer to one is that exchange's response. There is nothing
/// MCP-shaped about a status and some headers followed by a body, so
/// there is nothing here to define — see
/// [`http::response::Frame`](crate::shared::http::response::Frame) for what it
/// is and why it is split.
///
/// An alias rather than a re-export because this module is real. The
/// path says an MCP channel's response lives here, and it does, rather
/// than naming somewhere else and hoping a reader follows. What the
/// alias points at is right there in the signature.
pub type Frame<'a> = crate::shared::http::response::Frame<'a>;
