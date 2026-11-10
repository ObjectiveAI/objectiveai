//! `POST /dequeue`: the program's one slot, emptied.
//!
//! The proxy sends the [`request::Request`] JSON — `{}`, no key —
//! while a batch it offered is held: when a new message is to be
//! joined to it, or a key in it withdrawn. The program empties the
//! slot — the held `/enqueue` answers `dequeued` — and answers a
//! `2xx` with one [`Outcome`], whether it held anything. A slot the
//! loop already took stays taken: dequeuing is not un-delivery. The
//! proxy reads the slot's fate from the `/enqueue`'s answer, not from
//! this one, and offers its queue again, joined.

mod outcome;

pub mod request;

pub use outcome::*;
