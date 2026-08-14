//! Writing one file into a container.
//!
//! # Two channels, because only a responder can finish one
//!
//! A client opens a channel naming the destination. The provider then
//! opens a channel of its OWN asking for the bytes, and the client
//! streams them back as responses on it.
//!
//! That inversion is not decoration. There is no request-finish frame
//! in this protocol — [`ChannelResponseFinish`](crate::frame::client::ClientFrame::ChannelResponseFinish)
//! is sent by whoever is ANSWERING, so a side that is asking has no
//! way to say "that was the last one". A client streaming bytes as
//! repeated channel requests would have to invent an end marker in the
//! payload, and its request stream would become stateful: three
//! variants that only mean anything in sequence.
//!
//! Streaming as responses costs one round trip before the first byte
//! and buys a channel that does exactly one thing. It is the same
//! shape the OCI image pull already uses, for the same reason.
//!
//! # Piping a read into a write
//!
//! Which is the expected use, and it works without buffering. A
//! [`read`](super::read) body arrives as one frame; it goes out as one
//! [`bytes::Frame`]. No acknowledgement per chunk, no
//! length either end has to know in advance, and a consumer holds one
//! frame at a time however large the file is.
//!
//! # What a partial write leaves behind
//!
//! Nothing at the destination. A provider writes to a temporary in the
//! destination's own directory and renames it into place, so the path
//! holds the old file, then nothing, then the new one — never a
//! prefix of the new one.
//!
//! Where space is too tight for both copies, unlinking the old one
//! first frees exactly what the new one needs. That trades the old
//! contents away on failure, which is why it is worth doing only after
//! the ordinary attempt returns `ENOSPC` rather than up front.

pub mod bytes;
pub mod request;
pub mod response;
