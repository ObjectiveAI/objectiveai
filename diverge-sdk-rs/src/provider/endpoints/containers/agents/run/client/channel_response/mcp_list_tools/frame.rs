//! What a client's response frame carries on a tool listing channel.

/// The answer to a tool listing. See [`mcp::list_tools::response::Frame`](crate::shared::mcp::list_tools::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame = crate::shared::mcp::list_tools::response::Frame;
