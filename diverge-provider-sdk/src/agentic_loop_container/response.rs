//! What a container streams back.

/// One event of a container's response stream.
///
/// An alias, because what a container streams is the protocol's own
/// chunk vocabulary, one chunk per server-sent event — see
/// [`AgenticLoopChunk`](crate::endpoints::agentic_loop::run::server::response::AgenticLoopChunk)
/// for what it is.
///
/// An alias rather than a re-export because this module is real. The
/// path says the container's response lives here, and it does,
/// rather than naming somewhere else and hoping a reader follows.
/// What the alias points at is right there in the signature.
pub type Response =
    crate::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;
