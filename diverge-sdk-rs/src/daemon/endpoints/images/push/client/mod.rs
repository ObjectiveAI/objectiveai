//! The client side of an image push: what a client sends.
//!
//! [`request`] opens the scope; [`channel_response`] is what the
//! client answers on the channels the daemon opens — a manifest, a
//! blob, a proof. A client opens no channel of its own on a push, so
//! there is no `channel_request` here.
//!
//! # And a way to use it
//!
//! [`execute`] performs the exchange rather than describing it: hand
//! it a [`Handle`](crate::wire::client::handle::Handle), a
//! [`request::Frame`] and the caller's
//! [`OciStore`](crate::provider::client::OciStore), and read the
//! push's progress and its result off the stream it hands back, while
//! the daemon's asks are answered from the store.

pub mod channel_response;
pub mod request;

pub mod execute;
