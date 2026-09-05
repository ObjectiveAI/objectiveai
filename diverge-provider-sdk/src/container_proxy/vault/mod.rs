//! The vault: keys the caller holds, read, written and locked by
//! the container.
//!
//! A key-value store with locks, living with the caller. It is where
//! a container keeps what must outlive it — a cursor into an
//! external system, a thing two runs of one lineage must serialize
//! around: lock the key, read, act, write, unlock.
//!
//! The container asks with a [`Request`] on `/requests` (kind `4`),
//! and the server answers on `/vault/{channel}` with exactly one
//! message — a [`Response`], raw — and the close.
//!
//! ```text
//! the ask, after the channel:   [4][op: u8][key_len: u16 BE][key…][value…]
//! the answer, one message:      [kind: u8][payload…]
//! ```
//!
//! Binary throughout: keys are UTF-8 strings, values are bytes and
//! travel verbatim.
//!
//! # Keys are the container's; the namespace is the server's
//!
//! A key is whatever string the container chooses. WHICH vault it
//! lands in — whose, scoped how — the server decides from what it
//! knows about the container it is serving, and the wire carries no
//! namespace for the container to name. A container cannot reach
//! another's keys because it has no way to say them.
//!
//! # A lock is the connection's
//!
//! [`Lock`](Request::Lock) is answered when the lock is HELD —
//! however long that takes, since nothing times anything out — and
//! the holder is the `/requests` connection that asked. A second
//! `Lock` on a key that connection already holds is answered
//! [`Ok`](Response::Ok) at once; an [`Unlock`](Request::Unlock)
//! from a connection that does not hold the key is an
//! [`Error`](Response::Error). The `/requests` connection ending
//! RELEASES every lock it held. There are no lease ids: the
//! connection is the lease.
//!
//! # No retry
//!
//! An operation whose answer never came — `/requests` died before
//! the path opened, or the path died before its one message — is
//! reported to whoever asked as failed, never re-asked. A `Set`
//! re-sent might overwrite what another run wrote in between, and a
//! `Lock` re-sent after the old connection's locks were released is
//! a different lock.

mod request;
mod response;

pub use request::*;
pub use response::*;
