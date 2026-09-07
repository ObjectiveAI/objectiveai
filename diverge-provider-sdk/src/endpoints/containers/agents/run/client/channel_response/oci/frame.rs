//! What a client's response frame carries on an OCI channel.

/// One piece of a registry's answer, or the reason there is no more. See [`oci::response::Frame`](crate::shared::oci::response::Frame) for what it is and why it carries no head.
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::shared::oci::response::Frame<'a>;
