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
//! # FUSE mounts: files and directories the caller serves
//!
//! Beside the three paths the proxy MOUNTS: the server names
//! [`Mounts`] in the [`MOUNTS_ENV`] variable — the request's
//! [`fuse_file_mounts`](crate::shared::containers::request::Container::fuse_file_mounts)
//! and
//! [`fuse_directory_mounts`](crate::shared::containers::request::Container::fuse_directory_mounts),
//! handed down — and at its start the proxy mounts, at each path, a
//! FUSE filesystem whose contents are the CALLER's, asked by the
//! mount's id over [`fuse`](super::fuse): the caller serves the file
//! or the tree from wherever it keeps it, and nothing copies it in
//! or reads it back. It is for the credential files vendor CLIs
//! rewrite when they refresh a login. The mount needs `/dev/fuse` in
//! the container and the proxy running as root in the container's
//! user namespace, which is how the host runs it; a mount that
//! cannot be made ends the proxy at its start, as an unbindable port
//! does. The mount point — the file, or the directory — is made if
//! absent, every missing parent directory made first, and the
//! directory around it stays the image's own.
//!
//! ## A file mount
//!
//! A filesystem of exactly one regular file: the mount point is the
//! file itself.
//!
//! - The file is root's, one link, its size the caller's answer's
//!   length; mode `0600`, or `0400` when read-only. `stat` asks the
//!   caller; a caller that holds nothing under the id yet answers an
//!   empty file.
//! - `open` reads the file into a buffer of the handle's own, so a
//!   reader sees the snapshot its open took. No lock: the file is
//!   the caller's, and so is any serialization of it.
//! - OVERWRITABLE in place, every way a program overwrites a file it
//!   opens: `O_TRUNC` empties the handle's buffer; `truncate`
//!   resizes it (or, with no handle open, the file itself); `write`
//!   changes it at any offset, extending past the end; `flush`,
//!   `fsync` and the close of a changed handle store the whole buffer
//!   with the caller — a write the caller refuses is `EIO`, which is
//!   what the writer's `close` returns. Two write handles each store
//!   the whole buffer, and the last close wins. A `chmod` or `chown`
//!   is accepted and changes nothing.
//! - NOT deletable, NOT movable, and NOT replaceable by rename: the
//!   kernel refuses to unlink or rename a mount point (`EBUSY`), and
//!   a rename ONTO it is an operation on the directory around the
//!   mount, which is not the proxy's — so a program that saves by
//!   writing a temporary beside the file and renaming it over the
//!   file fails at the rename. Such a program gets a directory mount.
//!
//! ## A directory mount
//!
//! A real filesystem rooted at the path, the whole tree the caller's.
//!
//! - The root is a directory, root's, mode `0700` (`0500` read-only),
//!   that cannot be deleted or moved — it is the mount point. Every
//!   entry under it is the caller's: `lookup`, `stat` and `readdir`
//!   are a [`list`](super::fuse::list) of the entry's directory; a
//!   file's `open` is a [`read`](super::fuse::read) into a buffer of
//!   the handle's own, and its changed close a
//!   [`write`](super::fuse::write) of the whole buffer, exactly as a
//!   file mount's; `create` makes an empty file with the caller at
//!   once (so `O_EXCL` and a following `stat` see it) and opens it;
//!   `mkdir` is a [`mkdir`](super::fuse::mkdir); `unlink` and `rmdir`
//!   are a [`remove`](super::fuse::remove), a non-empty directory the
//!   caller's to refuse; `rename` is a [`rename`](super::fuse::rename)
//!   within the mount — `RENAME_NOREPLACE` honoured by a look first,
//!   `RENAME_EXCHANGE` `ENOTSUP`, a destination in another mount
//!   `EXDEV` — replacing a file at the destination whole, which is
//!   how a program that saves by temporary-and-rename overwrites.
//!   Symbolic links, hard links and device nodes are `EPERM`; times,
//!   mode and owner are accepted and changed nothing by; attributes
//!   are never cached, so every `stat` is the caller's current
//!   answer.
//! - Files are read and written whole, as on a file mount: the tree
//!   is for credentials and configuration, not data.
//!
//! ## Read-only
//!
//! Read-only means read-only: the mount carries the kernel's `ro`
//! option, so mutations are turned away before they reach the proxy,
//! AND every mutation path in the proxy answers `EROFS` regardless —
//! an open for writing or with `O_TRUNC`, a truncate, a write, a
//! create, a mkdir, an unlink, a rename. The caller never sees a
//! mutation for a read-only id.

pub mod read;
pub mod tree;
pub mod write;

mod mounts;

pub use mounts::*;
