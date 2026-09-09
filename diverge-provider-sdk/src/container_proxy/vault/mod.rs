//! The vault: keys the caller holds, read, written and locked by
//! the container.
//!
//! A key-value store with locks, living with the caller. It is where
//! a container keeps what must outlive it — a cursor into an
//! external system, a thing two runs of one lineage must serialize
//! around: lock the key, read, act, write, unlock.
//!
//! Five operations, each its own ask on `/requests` and its own
//! answer path, exactly as the MCP exchanges are:
//!
//! | ask | kind | payload after the kind | answered on | with |
//! |-----|------|------------------------|-------------|------|
//! | [`get`] | `5` | `[key…]` | `/vault/get/{channel}` | one [`get::response::Frame`] |
//! | [`set`] | `6` | `[key_len: u16 BE][key…][value…]` | `/vault/set/{channel}` | one [`response::Frame`] |
//! | [`delete`] | `7` | `[key…]` | `/vault/delete/{channel}` | one [`response::Frame`] |
//! | [`lock`] | `8` | `[ttl: u32 BE][key…]` | `/vault/lock/{channel}` | one [`response::Frame`] |
//! | [`unlock`] | `9` | `[key…]` | `/vault/unlock/{channel}` | one [`response::Frame`] |
//!
//! The answer is one message, raw, then the close: four of the five
//! share [`response::Frame`], ok or error, and a read has its own
//! three-way answer. A key is the
//! rest of the payload wherever nothing follows it, so only `Set`
//! carries a length prefix, and `Lock`'s TTL leads so its key can
//! be the rest too. Binary throughout: keys are UTF-8 strings,
//! values are bytes and travel verbatim.
//!
//! # Keys are the container's; the namespace is the server's
//!
//! A key is whatever string the container chooses. WHICH vault it
//! lands in — whose, scoped how — the server decides from what it
//! knows about the container it is serving, and the wire carries no
//! namespace for the container to name. A container cannot reach
//! another's keys because it has no way to say them.
//!
//! # The lock has a TTL, and the container is its holder
//!
//! [`lock`] is answered [`Ok`](response::Frame::Ok) when the lock is
//! HELD — however
//! long that takes, since nothing times anything out — and its TTL
//! runs from the grant. The holder is the CONTAINER, as the server
//! identifies it, not any connection: a `Lock` on a key the
//! container already holds refreshes the TTL to the new value and
//! is answered `Ok` at once, which is how a container keeps a lock
//! across a long job — send `Lock` again before the TTL runs out.
//! Expiry releases silently, and the next `Lock` from anyone wins.
//! [`unlock`] releases early; an `Unlock` from a container that
//! does not hold the key is [`Error`](response::Frame::Error). A TTL
//! of `0` is refused as `Error`.
//!
//! So `/requests` dying does NOT release a lock — the TTL does,
//! which is the point: a lock survives a reconnect and expires after
//! a crash, instead of vanishing the instant a connection does.
//!
//! # No retry
//!
//! An operation whose answer never came — `/requests` died before
//! the path opened, or the path died before its one message — is
//! reported to whoever asked as failed, never re-asked. A `Set`
//! re-sent might overwrite what another run wrote in between, and a
//! `Lock` re-sent after the fact might refresh a lock the container
//! had meanwhile decided to give up. The container decides.
//!
//! # Mounts: a key as one file
//!
//! Beside the five operations the proxy can serve a key AS A FILE: the
//! server names [`Mount`]s in the [`MOUNTS_ENV`] variable, and at its
//! start the proxy mounts, at each path, a FUSE filesystem of exactly
//! one regular file — the mount point is the file itself, made empty
//! if absent, and the directory around it stays the image's own. The
//! file is a vendor CLI's credential file, most often: a login the
//! CLI rewrites when it refreshes, which the caller keeps in the
//! vault and no harness has to copy in and read back.
//!
//! - The file is mode `0600`, root's, one link, its size the value's
//!   length. `stat` asks the vault; a missing key is an empty file.
//! - `open` reads the value into a buffer of the handle's own, so a
//!   reader sees the snapshot its open took. Opening for writing
//!   [`lock`]s the key first (TTL 300 s, not refreshed — a credential
//!   rewrite is milliseconds; a handle held past the TTL loses the
//!   lock and its write still lands); a lock refused is `EAGAIN`.
//!   `O_TRUNC` empties the buffer.
//! - `write` and `truncate` change the buffer; `flush`, `fsync` and
//!   the close of a changed handle [`set`] the whole buffer as the
//!   key's value — a set that fails is `EIO`, which is what the
//!   writer's `close` returns. Two write handles each set the whole
//!   buffer, and the last close wins. The close unlocks.
//! - It cannot be moved or deleted: the kernel refuses to rename or
//!   unlink a mount point (`EBUSY`), and nothing of the proxy's is
//!   asked. So a program that saves by writing a temp file beside and
//!   renaming over it fails at the rename; the mount is for programs
//!   that rewrite in place, which the credential files this exists
//!   for are.
//! - The mount needs `/dev/fuse` in the container and the proxy
//!   running as root in the container's user namespace, which is how
//!   the host runs it; a mount that cannot be made ends the proxy at
//!   its start, as an unbindable port does.

pub use crate::shared::containers::vault::*;

mod mounts;

pub use mounts::*;

// Each operation is a module of its own here, shadowing the shared
// one it re-exports, so the executor that answers it can live under
// it.
pub mod delete;
pub mod get;
pub mod lock;
pub mod set;
pub mod unlock;
