//! The client side of a volume write: what a client sends.
//!
//! [`request`] opens it — the volume and the destination, once.
//! [`channel_response`] answers the one channel the daemon opens:
//! the content, the provider's own frame re-exported.
//!
//! There is no `channel_request`. A client asks nothing more of a
//! write it started; the content is an answer, not an ask.

pub mod request;
pub use diverge_provider_sdk::endpoints::volumes::write::client::channel_response;
