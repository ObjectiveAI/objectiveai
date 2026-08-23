//! What a server's response frame carries on a resource read channel.

/// The answer to a resource read.
///
/// An alias, because what a channel carries here is what MCP says it
/// carries, and MCP says the same thing whichever direction the channel
/// runs. See
/// [`mcp::read_resource::response::Frame`](crate::shared::mcp::read_resource::response::Frame)
/// for what it is.
///
/// An alias rather than a re-export because this module is real. The
/// path says this endpoint's answer to a resource read lives here, and it
/// does, rather than naming somewhere else and hoping a reader follows.
/// What the alias points at is right there in the signature.
pub type Frame = crate::shared::mcp::read_resource::response::Frame;
