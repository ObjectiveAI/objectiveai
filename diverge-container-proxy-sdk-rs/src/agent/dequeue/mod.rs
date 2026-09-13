//! `POST /dequeue`: the running loop's queue, cleared.
//!
//! The proxy sends `{}` while a loop runs, after withdrawing every
//! message it still held itself. The program withdraws every message
//! it holds — each held `/enqueue` answers `dequeued` — and answers a
//! `2xx` with one [`Outcome`], whether it held anything. A message
//! the loop already took stays taken: dequeuing is not un-delivery.

mod outcome;

pub use outcome::*;
