//! What a container takes at `POST /`.

/// The one request a container serves.
///
/// An alias, because what arrives at the container's door is what
/// the client put on the protocol's wire — see
/// [`request::Frame`](crate::endpoints::agentic_loop::run::client::request::Frame)
/// for what it is. The server relays it as the bare JSON body; the
/// tag byte is the wire's, not HTTP's.
///
/// An alias rather than a re-export because this module is real. The
/// path says the container's request lives here, and it does, rather
/// than naming somewhere else and hoping a reader follows. What the
/// alias points at is right there in the signature.
pub type Request =
    crate::endpoints::agentic_loop::run::client::request::Frame;
