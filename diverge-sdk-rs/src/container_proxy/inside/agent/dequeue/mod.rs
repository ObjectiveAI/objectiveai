//! `POST /dequeue`: the messages waiting under a key, withdrawn.
//!
//! The proxy sends the [`request::Request`] JSON — a key, as the
//! enqueues gave it — while a loop runs, after withdrawing every
//! message under that key it still held itself. The program
//! withdraws every message it holds under the key — each held
//! `/enqueue` answers `dequeued` — leaves every other alone, and
//! answers a `2xx` with one [`Outcome`], whether it held any. A
//! message the loop already took stays taken: dequeuing is not
//! un-delivery.

mod outcome;

pub mod request;

pub use outcome::*;
