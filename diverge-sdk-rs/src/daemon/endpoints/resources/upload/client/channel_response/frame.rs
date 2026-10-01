//! What a client's channel response frame carries on an upload's
//! content channel.

/// A piece of the file, or a failure: the provider protocol's volume
/// write content answer, since the exchange is the same. See
/// [`volumes::write::client::channel_response::Frame`](crate::provider::endpoints::volumes::write::client::channel_response::Frame).
///
/// An alias rather than a re-export because this module is real: the
/// path says this scope's answer lives here, and it does, rather than
/// naming somewhere else and hoping a reader follows. What the alias
/// points at is right there in the signature.
pub type Frame<'a> = crate::provider::endpoints::volumes::write::client::channel_response::Frame<'a>;
