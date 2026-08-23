//! What a client's response frame carries on an OCI channel.

/// One piece of a registry's answer, or the reason there is no more.
///
/// An alias, because a registry answer is bytes off a socket and
/// nothing about relaying one changes between the endpoints that do
/// it. See
/// [`oci::response::Frame`](crate::shared::oci::response::Frame) for
/// what it is and why it carries no head.
///
/// An alias rather than a re-export because this module is real. The
/// path says this endpoint's registry answers live here, and they do,
/// rather than naming somewhere else and hoping a reader follows.
pub type Frame<'a> = crate::shared::oci::response::Frame<'a>;
