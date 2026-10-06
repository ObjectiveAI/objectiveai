//! What a client's channel response frame carries on an upload's
//! content channel.

/// A piece of the file, or a failure: the provider protocol's volume
/// write content answer, since the exchange is the same. See
/// [`volumes::write::client::channel_response::Frame`](crate::provider::endpoints::volumes::write::client::channel_response::Frame).
///
/// An alias rather than a re-export, as the resource upload's is: the
/// path says this scope's answer lives here, and it does.
pub type Frame<'a> = crate::provider::endpoints::volumes::write::client::channel_response::Frame<'a>;
