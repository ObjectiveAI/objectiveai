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
//! # Mounts: a file that can be overwritten but not moved or deleted
//!
//! Beside the three paths the proxy can MOUNT files: the server names
//! [`Mount`]s in the [`MOUNTS_ENV`] variable, and at its start the
//! proxy mounts, at each path, a FUSE filesystem of exactly one
//! regular file — the mount point is the file itself, made empty if
//! absent, and the directory around it stays the image's own. The
//! file's bytes are kept under a vault key, the one store the wire
//! has that outlives the container; the feature is the filesystem's.
//! It is for the credential files vendor CLIs rewrite when they
//! refresh a login: the caller keeps the file, and no harness copies
//! it in or reads it back.
//!
//! - The file is mode `0600`, root's, one link, its size the value's
//!   length. `stat` asks the vault; a missing key is an empty file.
//! - `open` reads the value into a buffer of the handle's own, so a
//!   reader sees the snapshot its open took. Opening for writing
//!   locks the key first (TTL 300 s, not refreshed — a credential
//!   rewrite is milliseconds; a handle held past the TTL loses the
//!   lock and its write still lands); a lock refused is `EAGAIN`.
//! - OVERWRITABLE, every way a program overwrites in place: `O_TRUNC`
//!   empties the handle's buffer; `truncate` resizes it (or, with no
//!   handle open, the value itself); `write` changes it at any
//!   offset, extending past the end; `flush`, `fsync` and the close
//!   of a changed handle set the whole buffer as the key's value — a
//!   set that fails is `EIO`, which is what the writer's `close`
//!   returns. Two write handles each set the whole buffer, and the
//!   last close wins. The close unlocks. A `chmod` or `chown` is
//!   accepted and changes nothing.
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
