//! The `/filesystem/read` path: one file, read out of the container.
//!
//! Opened by the server. It sends exactly one message — the
//! [`request::Request`], naming the file as path components from
//! the container's root — and the container answers with the file's
//! bytes, one [`Body`](response::Frame::Body) per message, each at
//! most [`CHUNK_SIZE`](crate::CHUNK_SIZE), then closes cleanly. A
//! read that cannot be served, or that fails partway, answers one
//! [`Error`](response::Frame::Error) — a reason, for a reader — as
//! its last message before the close.
//!
//! ```text
//! server → container:   [request JSON]                        once
//! container → server:   [0][bytes…] … [0][bytes…]             then the close
//!                    or [0][bytes…] … [1][message…]           then the close
//! ```
//!
//! # How it ends
//!
//! | the path ends with | means |
//! |--------------------|-------|
//! | bodies, then the close | the bytes are the file |
//! | an `Error`, then the close | the file was not read, or not all of it, and this is why |
//! | the close, with nothing before it | could not serve, with nothing to say — the standing meaning on every path |
//!
//! A file of zero bytes is one empty body, then the close: the empty
//! close keeps its standing meaning, so an empty file has to say it
//! is one. Nothing here carries a length, a `Complete` or a count —
//! see [`shared::containers::read`](crate::shared::containers::read)
//! for why a directory is never read and for the silent tear a
//! reader of a live file accepts.
//!
//! # The error propagates
//!
//! What the container says here is what the provider answers the
//! caller with: a
//! [`read`](crate::shared::containers::read) on a container scope
//! ends in an `Error` carrying the same reason. The container speaks
//! a string because that is what a filesystem gives it; the provider
//! carries it as the JSON value its wire uses.
//!
//! # No retry
//!
//! An abrupt end is a read that died, reported as such to whoever
//! asked. The server may ask again; nothing here does it for them.

pub mod request;
pub mod response;

#[cfg(feature = "server")]
pub mod execute;
