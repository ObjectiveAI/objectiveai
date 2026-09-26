//! FUSE mounts: files and directories the caller serves live.
//!
//! A [`FuseMount`](super::request::FuseMount) on a container request
//! names a path and an id the caller minted — on one of two lists, a
//! FILE mount or a DIRECTORY mount. The provider mounts, at that path, a FUSE filesystem the
//! proxy inside the container serves — of exactly one regular file,
//! or of a whole directory tree — and everything behind it is the
//! CALLER's: every `stat` asks what an entry is with [`stat`], every
//! `read(2)` reads its piece with [`read`], every `write(2)` lands
//! its piece with [`mod@write`], a truncation is [`truncate`], a
//! change of mode, owner or times is [`setattr`], and in a directory
//! mount every listing, creation, deletion and rename is an ask of
//! its own. Each ask carries the mount's id and the entry's path
//! RELATIVE to the mount root — `/`-separated UTF-8, no leading
//! slash, EMPTY for a file mount and for the directory root — so one
//! vocabulary serves both kinds.
//!
//! Nine operations, each its own ask and its own one-message
//! answer:
//!
//! | ask | payload | answered with |
//! |-----|---------|---------------|
//! | [`stat`] | `[id_len: u16 BE][id…][path…]` | one [`stat::response::Frame`]: `0` a [`Stat`](stat::Stat), fifty-seven bytes, `1` missing, `2` error |
//! | [`read`] | `[id_len: u16 BE][id…][path_len: u16 BE][path…][offset: u64 BE][length: u32 BE]` | one [`read::response::Frame`]: `0` the piece, `1` missing, `2` error |
//! | [`mod@write`] | `[id_len: u16 BE][id…][path_len: u16 BE][path…][offset: u64 BE][bytes…]` | one [`Ack`](ack::Frame): `0` ok, `1` error, `2` read only |
//! | [`truncate`] | `[id_len: u16 BE][id…][size: u64 BE][path…]` | one [`Ack`](ack::Frame) |
//! | [`setattr`] | `[id_len: u16 BE][id…][attrs: 29 bytes][path…]` | one [`Ack`](ack::Frame) |
//! | [`list`] | `[id_len: u16 BE][id…][path…]` | one [`list::response::Frame`]: `0` the entries, each `[kind: u8][name_len: u16 BE][name…]`, `1` missing, `2` error |
//! | [`remove`] | `[id_len: u16 BE][id…][path…]` | one [`Ack`](ack::Frame) |
//! | [`rename`] | `[id_len: u16 BE][id…][from_len: u16 BE][from…][to…]` | one [`Ack`](ack::Frame) |
//! | [`mkdir`] | `[id_len: u16 BE][id…][path…]` | one [`Ack`](ack::Frame) |
//!
//! Where a path is the last thing in a payload it runs to the end;
//! where bytes, fixed fields or a second path follow it, it carries a
//! length prefix. Binary throughout: ids and paths are UTF-8 strings,
//! bytes travel verbatim. The same shapes ride both wires this crate
//! defines: the provider's channel toward the caller, and the
//! [`proxy`](crate::container_proxy_endpoints::fuse) inside the
//! container — and, the id left off, a caller's channels on a
//! [`volumes::serve`](crate::endpoints::volumes::serve) scope, where
//! a provider answers them from a volume of its own.
//!
//! # The id is the caller's, and opaque
//!
//! Whatever string the caller put on the mount. Nothing between the
//! container and the caller reads it, and a container cannot name a
//! mount the caller did not make, because it has no way to say an id
//! it was not given.
//!
//! # Every piece crosses as it is
//!
//! Nothing is buffered on either side. The kernel hands the mount a
//! `read(2)` or a `write(2)` with an offset and a size, and the mount
//! sends exactly that as one ask, at most
//! [`CHUNK_SIZE`](crate::CHUNK_SIZE) at a time, and answers the
//! program with what came back: a file of any size crosses in the
//! pieces the program asks for, a change lands where it is made, and
//! nothing waits for a close — a `flush` and an `fsync` cost no ask.
//! A file gets shorter only by [`truncate`], and a `create` is a
//! [`mod@write`] of nothing at offset zero. What is NOT a read is a
//! `stat`: the kernel asks for an entry's attributes far more often
//! than for its bytes — on every `stat(2)`, before most opens, once
//! per component of every path it resolves — and [`stat`] answers
//! those with fifty-seven bytes, the kind, the size, the mode, the
//! owner, the group and the three times, so a file's bytes cross the
//! wire only when a program reads them. A [`list`] likewise carries
//! only what a `readdir` shows, names and kinds; an entry's
//! attributes are its own `stat`. The price is a round trip per
//! piece, which is why a mount is for the files a program edits and a
//! volume mount on the provider the container runs on is for the
//! data it churns.
//!
//! # A read-only volume
//!
//! A mount served from a volume whose mode is
//! [`ReadOnly`](crate::endpoints::volumes::Mode::ReadOnly) takes no
//! change. Every mutating ask on it — a write, a truncate, a setattr,
//! a remove, a rename, a mkdir — is answered
//! [`ReadOnly`](ack::Frame::ReadOnly), which the program sees as a
//! read-only filesystem, and every immutable ask goes through. An
//! ephemeral volume takes every change and discards it afterwards,
//! and a persistent one keeps it; neither refuses a change for its
//! mode.
//!
//! # A file mount is one file; a directory mount is a tree
//!
//! On a file mount the path is always empty, and the file can be read
//! and written in place — opened, truncated, written, closed — but
//! never deleted or moved: the mount point is the file itself,
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
//! # Refusal is the caller's
//!
//! The proxy sends every mutation the program attempts — a `write`,
//! a `truncate`, a `setattr`, a `remove`, a `rename`, a `mkdir` — and
//! a caller that will not have one answers the error. The program sees the operation fail,
//! and nothing between them enforces a policy of its own: a mount
//! the caller keeps unchangeable is one whose every mutation it
//! refuses.
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
pub mod setattr;
pub mod stat;
pub mod truncate;
pub mod write;

mod entry;
mod error;
mod listed;
mod piece;
mod prefixed;
mod target;
mod time;

pub use entry::*;
pub use error::*;
pub use listed::*;
pub use setattr::Attrs;
pub use target::*;
pub use time::*;
