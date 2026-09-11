//! FUSE mounts: files and directories the caller serves live.
//!
//! A [`FuseMount`](super::request::FuseMount) on a container request
//! names a path, an id the caller minted, and whether the mount is
//! read-only — on one of two lists, a FILE mount or a DIRECTORY
//! mount. The provider mounts, at that path, a FUSE filesystem the
//! proxy inside the container serves — of exactly one regular file,
//! or of a whole directory tree — and everything behind it is the
//! CALLER's: every `stat` asks what an entry is with [`stat`], every
//! open reads a file's bytes with [`read`], every changed close
//! stores them with [`mod@write`], and in a directory mount every
//! listing, creation, deletion and rename is an ask of its own. Each ask carries the mount's id and the entry's path
//! RELATIVE to the mount root — `/`-separated UTF-8, no leading
//! slash, EMPTY for a file mount and for the directory root — so one
//! vocabulary serves both kinds.
//!
//! Seven operations, each its own ask and its own one-message
//! answer:
//!
//! | ask | payload | answered with |
//! |-----|---------|---------------|
//! | [`stat`] | `[id_len: u16 BE][id…][path…]` | one [`stat::response::Frame`]: `0` `[kind: u8][size: u64 BE]`, `1` missing, `2` error |
//! | [`read`] | `[id_len: u16 BE][id…][path…]` | one [`read::response::Frame`]: `0` the bytes, `1` missing, `2` error |
//! | [`mod@write`] | `[id_len: u16 BE][id…][path_len: u16 BE][path…][bytes…]` | one [`Ack`](ack::Frame): `0` ok, `1` error |
//! | [`list`] | `[id_len: u16 BE][id…][path…]` | one [`list::response::Frame`]: `0` the entries, each `[kind: u8][name_len: u16 BE][name…]`, `1` missing, `2` error |
//! | [`remove`] | `[id_len: u16 BE][id…][path…]` | one [`Ack`](ack::Frame) |
//! | [`rename`] | `[id_len: u16 BE][id…][from_len: u16 BE][from…][to…]` | one [`Ack`](ack::Frame) |
//! | [`mkdir`] | `[id_len: u16 BE][id…][path…]` | one [`Ack`](ack::Frame) |
//!
//! Where a path is the last thing in a payload it runs to the end;
//! where bytes or a second path follow it, it carries a length
//! prefix. Binary throughout: ids and paths are UTF-8 strings, bytes
//! travel verbatim. The same shapes ride both wires this crate
//! defines: the provider's channel toward the caller, and the
//! [`proxy`](crate::container_proxy::fuse) inside the container.
//!
//! # The id is the caller's, and opaque
//!
//! Whatever string the caller put on the mount. Nothing between the
//! container and the caller reads it, and a container cannot name a
//! mount the caller did not make, because it has no way to say an id
//! it was not given.
//!
//! # A file is one message, and a stat is nine bytes
//!
//! A read answers the whole file in one message and a write carries
//! the whole file in one ask, so a FUSE mount is for files the size
//! of a credential or a configuration, not a database; the message
//! cap is the transport's. Reads are not paged and writes are not
//! chunked, by design: a file is replaced whole on every changed
//! close, which is the only write a mount ever makes. What is NOT a
//! read is a `stat`: the kernel asks for an entry's attributes far
//! more often than for its bytes — on every `stat(2)`, before most
//! opens, once per component of every path it resolves — and [`stat`]
//! answers those with the kind and the size alone, so a file's bytes
//! cross the wire only when a program opens it. A [`list`] likewise
//! carries only what a `readdir` shows, names and kinds; the size of
//! an entry is its own `stat`.
//!
//! # A file mount is one file; a directory mount is a tree
//!
//! On a file mount the path is always empty, and the file can be read
//! and overwritten in place — opened, truncated, written, closed —
//! but never deleted or moved: the mount point is the file itself,
//! and the kernel refuses to unlink or rename a mount point. A
//! program that saves by writing a temporary beside the file and
//! renaming it over the file fails at the rename, because that rename
//! is an operation on the directory around the mount, which is not
//! the proxy's. Such a program gets a DIRECTORY mount instead: there
//! the whole tree is the caller's, every entry can be created,
//! overwritten by either method, renamed and deleted, and only the
//! root — the mount point — is fixed. [`stat`] answers what one entry
//! is, and the proxy answers the root itself; [`list`] answers the
//! entries of a directory with their kind; [`remove`] takes a file or
//! an empty directory, and a non-empty one is the caller's to
//! refuse; [`rename`] moves within one mount and replaces a file at
//! its destination, a directory there being the caller's to refuse;
//! [`mkdir`] makes one directory under an existing one. A path is
//! the caller's to validate — it has no `..` and no empty component
//! as the proxy sends it — and one the caller will not serve is
//! answered with the error.
//!
//! # Read-only never writes
//!
//! A mount named read-only refuses every mutation inside the
//! container, so the caller never sees a `write`, `remove`, `rename`
//! or `mkdir` for that id. A caller that gets one anyway — a
//! container that ignored the flag — may answer the error.
//!
//! # No retry
//!
//! An operation whose answer never came is reported to whoever asked
//! as failed, never re-asked: a `write` or a `rename` re-sent might
//! undo what the caller did in between.

pub mod ack;
pub mod list;
pub mod mkdir;
pub mod read;
pub mod remove;
pub mod rename;
pub mod stat;
pub mod write;

mod entry;
mod error;
mod prefixed;
mod target;

pub use entry::*;
pub use error::*;
pub use target::*;
