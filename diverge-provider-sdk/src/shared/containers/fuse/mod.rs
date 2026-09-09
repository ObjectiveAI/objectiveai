//! FUSE mounts: files the caller serves live, one file each.
//!
//! A [`FuseMount`](super::request::FuseMount) on a container request
//! names a path, an id the caller minted, and whether the file is
//! read-only. The provider mounts, at that path, a FUSE filesystem of
//! exactly one regular file — the proxy inside the container does, at
//! its start — and the file's bytes are the CALLER's: every open
//! reads them with [`read`], every changed close stores them with
//! [`mod@write`], each ask carrying the id, and the caller answers from
//! wherever it keeps that file.
//!
//! Two operations, each its own ask and its own one-message answer:
//!
//! | ask | payload | answered with |
//! |-----|---------|---------------|
//! | [`read`] | `[id…]` | one [`read::response::Frame`] |
//! | [`mod@write`] | `[id_len: u16 BE][id…][bytes…]` | one [`write::response::Frame`] |
//!
//! The id is the rest of the payload where nothing follows it, so
//! only `write` carries a length prefix. Binary throughout: ids are
//! UTF-8 strings the caller chose, bytes travel verbatim. The same
//! shapes ride both wires this crate defines: the provider's channel
//! toward the caller, and the [`proxy`](crate::container_proxy::fuse)
//! inside the container.
//!
//! # The id is the caller's, and opaque
//!
//! Whatever string the caller put on the mount. Nothing between the
//! container and the caller reads it, and a container cannot name a
//! file the caller did not mount, because it has no way to say an id
//! it was not given.
//!
//! # A file is one message
//!
//! A read answers the whole file in one message and a write carries
//! the whole file in one ask, so a FUSE mount is for files the size
//! of a credential or a configuration, not a database; the message
//! cap is the transport's. Reads are not paged and writes are not
//! chunked, by design: the file is replaced whole on every changed
//! close, which is the only write a mount ever makes.
//!
//! # Read-only never writes
//!
//! A mount named read-only refuses every write inside the container,
//! so the caller never sees a `write` for that id. A caller that gets
//! one anyway — a container that ignored the flag — may answer
//! [`Error`](write::response::Frame::Error).
//!
//! # No retry
//!
//! An operation whose answer never came is reported to whoever asked
//! as failed, never re-asked: a `write` re-sent might overwrite what
//! the caller wrote in between.

pub mod read;
pub mod write;

mod error;

pub use error::*;
