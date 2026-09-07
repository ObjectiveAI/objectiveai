//! What a server's response frame carries on an agentic loop channel.

/// One chunk of the loop, or the news that there will not be one. See [`agentic_loop::response::Frame`](crate::shared::containers::agentic_loop::response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame = crate::shared::containers::agentic_loop::response::Frame;
