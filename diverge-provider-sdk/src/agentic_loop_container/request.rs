//! What a container takes first on the socket at `/`.

/// The one request a container serves.
///
/// An alias, because what arrives at the container's door is what
/// the client put on the protocol's wire — see
/// [`request::Frame`](crate::endpoints::agentic_loop::run::client::request::Frame)
/// for what it is. It arrives as the FIRST binary message on the
/// WebSocket the server opens at `/`: the wire's request frame
/// verbatim, its tag byte included, so the server relays the bytes
/// it received without decoding them, and the container decodes them
/// with the frame's own [`Decode`](crate::decode::Decode).
///
/// An alias rather than a re-export because this module is real. The
/// path says the container's request lives here, and it does, rather
/// than naming somewhere else and hoping a reader follows. What the
/// alias points at is right there in the signature.
pub type Request =
    crate::endpoints::agentic_loop::run::client::request::Frame;
