//! Asking for a write's content, and streaming it.
//!
//! The second half of a write, and the one that travels the other way.
//! A provider opens a channel with a [`request::Request`] naming which
//! write it wants content for, and the client streams
//! [`response::Frame`]s back until it finishes the channel.
//!
//! See [`write_path`](super::write_path) for why the content is
//! collected this way round rather than pushed with the destination.
//!
//! # Piping a read into a write
//!
//! Which is the expected use, and it works without buffering. A
//! [`read`](super::read) body arrives as one frame and goes out as one
//! [`response::Frame::Body`]. No acknowledgement per chunk, no length
//! either end has to know in advance, and a consumer holds one frame
//! at a time however large the file is.
//!
//! The read has no error to translate. It reports a failure by
//! stopping, and a client piping one can turn that into a
//! [`response::Frame::Error`] or simply stop too — see
//! [`response::Frame`] for why both remain.

pub mod request;
pub mod response;
