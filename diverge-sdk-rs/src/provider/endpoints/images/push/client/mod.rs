//! The client side of an image push: what a client sends.
//!
//! [`request`] opens the scope; [`channel_response`] is what the
//! client answers on the channels the provider opens — a manifest, a
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
//! the provider's asks are answered from the store.
//!
//! Every other module in [`endpoints`](crate::provider::endpoints) is types only,
//! and this one still is unless a caller asked for the half of the
//! crate that can hold a socket — the same bargain
//! [`client`](crate::wire::client) itself makes.

pub mod channel_response;
pub mod request;

pub mod execute;
