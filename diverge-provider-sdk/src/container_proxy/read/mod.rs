//! The `/read` path: one file, read out of the container.
//!
//! Opened by the server. It sends exactly one message — the
//! [`request::Request`], naming the file as path components from
//! the container's root — and the container answers with the file's
//! bytes, one [`response::Frame`] per message, each at most
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE), then closes cleanly.
//! The clean close is the read complete: there is no length, no
//! `Complete` and no failure frame, which is the laboratories' read
//! exactly — see [`shared::container::read`](crate::shared::container::read)
//! for why a directory is never read and for the silent tear a
//! reader of a live file accepts.
//!
//! ```text
//! server → container:   [request JSON]           once
//! container → server:   [bytes…] … [bytes…]     then the close
//! ```
//!
//! # An empty file is one empty message
//!
//! A clean close with no message before it keeps its standing
//! meaning on every path — the container could not serve the ask —
//! so a file of zero bytes is answered with one message carrying no
//! bytes, then the close. No reason travels with a refusal; the
//! laboratories read has none either.
//!
//! # No retry
//!
//! An abrupt end is a read that died, reported as such to whoever
//! asked. The server may ask again; nothing here does it for them.

pub mod request;
pub mod response;
