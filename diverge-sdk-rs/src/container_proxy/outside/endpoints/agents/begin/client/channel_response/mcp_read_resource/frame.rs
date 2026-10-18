//! What a client's response frame carries on a resource read channel.

/// The answer to a resource read. See [`mcp::read_resource::response::Frame`](crate::shared::mcp::read_resource::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame = crate::shared::mcp::read_resource::response::Frame;
