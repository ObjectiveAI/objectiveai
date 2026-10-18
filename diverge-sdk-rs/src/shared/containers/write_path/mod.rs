//! Naming the file to write, and hearing whether it landed.
//!
//! The first half of a write. A client opens a channel with a
//! [`request::Request`], and the provider answers on that same channel
//! with a [`response::Frame`] once it knows.
//!
//! The content does not travel here. It travels on a second channel
//! the PROVIDER opens — see [`write_bytes`](super::write_bytes).
//!
//! # Why a write is two exchanges
//!
//! Because only a responder can end a channel.
//! [`ChannelResponseFinish`](crate::wire::frame::client::ClientFrame::ChannelResponseFinish)
//! is sent by whoever is ANSWERING, so a side that is asking has no
//! way to say "that was the last one". A client streaming content as
//! repeated channel requests would have to invent an end marker in the
//! payload, and its request stream would become stateful: variants
//! that only mean anything in sequence.
//!
//! Inverting the second exchange fixes that with machinery that
//! already exists. It costs one round trip before the first byte and
//! buys two channels that each do exactly one thing — which is the
//! same shape the OCI image pull uses, for the same reason.

pub mod request;
pub mod response;
