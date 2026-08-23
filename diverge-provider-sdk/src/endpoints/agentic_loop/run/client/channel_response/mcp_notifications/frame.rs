//! What a client's response frame carries on the notification stream channel.

/// The answer to the notification stream.
///
/// An alias, because what a channel carries here is what MCP says it
/// carries, and MCP says the same thing whichever direction the channel
/// runs. See
/// [`mcp::notifications::response::Frame`](crate::shared::mcp::notifications::response::Frame)
/// for what it is.
///
/// An alias rather than a re-export because this module is real. The
/// path says this endpoint's answer to the notification stream lives here, and it
/// does, rather than naming somewhere else and hoping a reader follows.
/// What the alias points at is right there in the signature.
pub type Frame = crate::shared::mcp::notifications::response::Frame;
