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
//! | [`Get`] | `5` | `[key…]` | `/vault/get/{channel}` | one [`Value`] |
//! | [`Set`] | `6` | `[key_len: u16 BE][key…][value…]` | `/vault/set/{channel}` | one [`Done`] |
//! | [`Delete`] | `7` | `[key…]` | `/vault/delete/{channel}` | one [`Done`] |
//! | [`Lock`] | `8` | `[ttl: u32 BE][key…]` | `/vault/lock/{channel}` | one [`Done`] |
//! | [`Unlock`] | `9` | `[key…]` | `/vault/unlock/{channel}` | one [`Done`] |
//!
//! The answer is one message, raw, then the close. A key is the
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
//! [`Lock`] is answered [`Done::Ok`] when the lock is HELD — however
//! long that takes, since nothing times anything out — and its TTL
//! runs from the grant. The holder is the CONTAINER, as the server
//! identifies it, not any connection: a `Lock` on a key the
//! container already holds refreshes the TTL to the new value and
//! is answered `Ok` at once, which is how a container keeps a lock
//! across a long job — send `Lock` again before the TTL runs out.
//! Expiry releases silently, and the next `Lock` from anyone wins.
//! [`Unlock`] releases early; an `Unlock` from a container that
//! does not hold the key is [`Done::Error`]. A TTL of `0` is
//! refused as `Error`.
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

mod delete;
mod done;
mod error;
mod get;
mod lock;
mod set;
mod unlock;
mod value;

pub use delete::*;
pub use done::*;
pub use error::*;
pub use get::*;
pub use lock::*;
pub use set::*;
pub use unlock::*;
pub use value::*;
