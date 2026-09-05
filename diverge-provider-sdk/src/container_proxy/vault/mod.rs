//! The `/vault` path: keys the caller holds, read, written and
//! locked by the container.
//!
//! A key-value store with locks, living with the caller. It is where
//! a container keeps what must outlive it and must never be rewound
//! — a credential that rotates, a cursor into an external system —
//! and where two runs of one lineage serialize around such a thing:
//! lock the key, refresh, write, unlock.
//!
//! An exchange path (see [the module](super)): the container opens a
//! channel with one [`Request`](request::Request), the server
//! answers with exactly one [`Response`](response::Response) and the
//! finish, and the channel is dead.
//!
//! ```text
//! container → server:  [channel: u8][kind: u8][key_len: u16 BE][key…][value…]
//! server → container:  [type: u8][channel: u8][kind: u8][payload…]
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
//! [`Lock`](request::Request::Lock) is answered when the lock is
//! HELD — however long that takes, since nothing times anything out
//! — and the holder is this WebSocket connection. A second `Lock`
//! on a key this connection already holds is answered
//! [`Ok`](response::Response::Ok) at once; an
//! [`Unlock`](request::Request::Unlock) from a connection that does
//! not hold the key is an [`Error`](response::Response::Error). The
//! connection ending RELEASES every lock it held. There are no lease
//! ids: the connection is the lease, which is the whole reason the
//! proxy holds one connection per path.
//!
//! # No retry law
//!
//! Unlike [`mcp`](super::mcp), a channel that died with its
//! connection is NOT asked again. A `Set` re-sent might overwrite
//! what another run wrote in between, and a `Lock` re-sent after the
//! old connection's locks were released is a different lock. The
//! request is reported to whoever asked as failed, and they decide.
//!
//! # Types
//!
//! The type space is the server's alone — the container's one frame
//! carries none; its request kind is inside the payload.
//!
//! | type | server |
//! |------|--------|
//! | 0    | channel response |
//! | 1    | channel response finish |

pub mod request;
pub mod response;

mod error;

pub use error::{FrameError, HEADER_LEN};
use error::split_header;
