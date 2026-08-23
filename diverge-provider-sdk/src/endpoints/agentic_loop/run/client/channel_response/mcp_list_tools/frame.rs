//! What a client's response frame carries on a tool listing channel.

/// The answer to a tool listing.
///
/// An alias, because what a channel carries here is what MCP says it
/// carries, and MCP says the same thing whichever direction the channel
/// runs. See
/// [`mcp::list_tools::response::Frame`](crate::shared::mcp::list_tools::response::Frame)
/// for what it is.
///
/// An alias rather than a re-export because this module is real. The
/// path says this endpoint's answer to a tool listing lives here, and it
/// does, rather than naming somewhere else and hoping a reader follows.
/// What the alias points at is right there in the signature.
pub type Frame = crate::shared::mcp::list_tools::response::Frame;
