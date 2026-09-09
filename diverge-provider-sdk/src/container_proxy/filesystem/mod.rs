//! The `/filesystem/*` paths: the server's three ways into the
//! container's filesystem.
//!
//! | path | carries |
//! |------|---------|
//! | [`/filesystem/tree`](tree) | the tree, watched: a snapshot, then every change, or why there is none |
//! | [`/filesystem/read`](read) | one file out: the server names it, the container answers its bytes, or why not |
//! | [`/filesystem/write`](mod@write) | one file in: the server names it and sends its content, the container answers ok or error |
//!
//! All three are opened by the server, as many times as it likes —
//! each a subscription, each one file — and nothing is shared
//! between openings. The shapes are the wire's own,
//! [`shared::containers`](crate::shared::containers)' `filetree`,
//! `read`, `write_path` and `write_bytes`, re-exported or wrapped by
//! each path with the one thing this wire adds: a way to say why
//! there will not be one.
//!
//! # FUSE mounts: a file the caller serves, overwritable unless read-only
//!
//! Beside the three paths the proxy MOUNTS files: the server names
//! [`Mount`]s in the [`MOUNTS_ENV`] variable — the request's
//! [`FuseMount`](crate::shared::containers::request::FuseMount)s, handed
//! down — and at its start the proxy mounts, at each path, a FUSE
//! filesystem of exactly one regular file: the mount point is the
//! file itself, made empty if absent — every missing parent directory
//! made first — and the directory around it stays the image's own. The file's bytes are the CALLER's, asked by
//! the mount's id over [`fuse`](super::fuse) — a read on every open,
//! a write on every changed close — so the caller serves the file
//! from wherever it keeps it, and nothing copies it in or reads it
//! back. It is for the credential files vendor CLIs rewrite when they
//! refresh a login.
//!
//! - The file is root's, one link, its size the caller's answer's
//!   length; mode `0600`, or `0400` when read-only. `stat` asks the
//!   caller; a caller that holds nothing under the id yet answers an
//!   empty file.
//! - `open` reads the file into a buffer of the handle's own, so a
//!   reader sees the snapshot its open took. No lock: the file is
//!   the caller's, and so is any serialization of it.
//! - OVERWRITABLE, every way a program overwrites in place: `O_TRUNC`
//!   empties the handle's buffer; `truncate` resizes it (or, with no
//!   handle open, the file itself); `write` changes it at any
//!   offset, extending past the end; `flush`, `fsync` and the close
//!   of a changed handle store the whole buffer with the caller — a
//!   write the caller refuses is `EIO`, which is what the writer's
//!   `close` returns. Two write handles each store the whole buffer,
//!   and the last close wins. A `chmod` or `chown` is accepted and
//!   changes nothing.
//! - READ-ONLY means read-only: the mount carries the kernel's `ro`
//!   option, so writes are turned away before they reach the proxy,
//!   AND every write path in the proxy answers `EROFS` regardless —
//!   an open for writing or with `O_TRUNC`, a truncate, a write. The
//!   caller never sees a write for a read-only id.
//! - NOT movable, NOT deletable: the kernel refuses to rename or
//!   unlink a mount point (`EBUSY`), and nothing of the proxy's is
//!   asked. A program that saves by writing a temp file beside and
//!   renaming over it fails at the rename; overwrite in place instead,
//!   which is what the credential files this exists for do.
//! - The mount needs `/dev/fuse` in the container and the proxy
//!   running as root in the container's user namespace, which is how
//!   the host runs it; a mount that cannot be made ends the proxy at
//!   its start, as an unbindable port does.

pub mod read;
pub mod tree;
pub mod write;

mod mounts;

pub use mounts::*;
