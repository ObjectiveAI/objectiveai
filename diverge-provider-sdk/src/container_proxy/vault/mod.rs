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

pub use crate::shared::containers::vault::*;
