//! The vault: keys the caller holds, read, written and locked by the
//! container.
//!
//! A key-value store with locks, living with the caller. It is where a
//! container keeps what must outlive it — a cursor into an external
//! system, a thing two runs of one lineage must serialize around:
//! lock the key, read, act, write, unlock.
//!
//! Five operations, each its own ask and its own one-message answer:
//!
//! | ask | payload | answered with |
//! |-----|---------|---------------|
//! | [`get`] | `[key…]` | one [`get::response::Frame`] |
//! | [`set`] | `[key_len: u16 BE][key…][value…]` | one [`response::Frame`] |
//! | [`delete`] | `[key…]` | one [`response::Frame`] |
//! | [`lock`] | `[ttl: u32 BE][key…]` | one [`response::Frame`] |
//! | [`unlock`] | `[key…]` | one [`response::Frame`] |
//!
//! Four of the five share [`response::Frame`], ok or error, and a read
//! has its own three-way answer. A key is the rest of the payload
//! wherever nothing follows it, so only `set` carries a length prefix,
//! and `lock`'s TTL leads so its key can be the rest too. Binary
//! throughout: keys are UTF-8 strings, values are bytes and travel
//! verbatim. The same shapes ride both wires this crate defines: the
//! provider's channel toward the caller, and the
//! [`proxy`](crate::container_proxy::outside::endpoints::agents::begin) inside the container.
//!
//! # Keys are the container's; the namespace is the caller's
//!
//! A key is whatever string the container chooses. WHICH vault it
//! lands in — whose, scoped how — is decided by who is serving the
//! container, and the wire carries no namespace for the container to
//! name. A container cannot reach another's keys because it has no
//! way to say them.
//!
//! # The lock has a TTL, and the container is its holder
//!
//! [`lock`] is answered [`Ok`](response::Frame::Ok) when the lock is
//! HELD — however long that takes, since nothing times anything out —
//! and its TTL runs from the grant. The holder is the CONTAINER, not
//! any connection: a `lock` on a key the container already holds
//! refreshes the TTL to the new value and is answered `Ok` at once,
//! which is how a container keeps a lock across a long job — lock
//! again before the TTL runs out. Expiry releases silently, and the
//! next `lock` from anyone wins. [`unlock`] releases early; an unlock
//! from a container that does not hold the key is
//! [`Error`](response::Frame::Error). A TTL of `0` is refused as
//! `Error`.
//!
//! So a connection dying does NOT release a lock — the TTL does, which
//! is the point: a lock survives a reconnect and expires after a
//! crash, instead of vanishing the instant a connection does.
//!
//! # No retry
//!
//! An operation whose answer never came is reported to whoever asked
//! as failed, never re-asked. A `set` re-sent might overwrite what
//! another run wrote in between, and a `lock` re-sent after the fact
//! might refresh a lock the container had meanwhile decided to give
//! up. The container decides.

pub mod delete;
pub mod get;
pub mod keys;
pub mod lock;
pub mod response;
pub mod set;
pub mod unlock;

mod error;

pub use error::*;
