//! The `/write` path: one file, written into the container.
//!
//! Opened by the server. It sends the [`request::Request`] as the
//! first message — the destination, as path components from the
//! container's root — then the content as [`request::Frame`]s, one
//! chunk per message, then ONE EMPTY MESSAGE: the end of the
//! content. The container finishes the file, answers with one
//! [`response::Frame`] — `Ok`, or `Error` with a reason — and closes
//! cleanly.
//!
//! ```text
//! server → container:   [request JSON] [bytes…] … [bytes…] []
//! container → server:   [kind: u8][message…]      then the close
//! ```
//!
//! # Why an empty message ends the content
//!
//! The laboratories split a write in two because only a responder
//! could end a channel, and the side streaming content had no way to
//! say "that was the last byte". Here the server needs to say it on
//! a socket it still wants an answer on, so closing will not do. An
//! empty chunk carries nothing as content, so it is free to carry
//! this — no tag on every chunk, no second exchange.
//!
//! # What lands
//!
//! The file, created or replaced, as the bytes arrived; parents are
//! not created — the ask names a file, not a tree, and a missing
//! parent is an `Error`. A server that closes before the empty
//! message has ABANDONED the write, and the container discards what
//! it wrote: a half-written file that looks finished is the worse
//! outcome. An abrupt end from either side is a write that died,
//! reported the same way. Nothing here retries.

pub mod request;
pub mod response;
