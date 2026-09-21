//! `POST /enqueue`: a message for the running loop's queue.
//!
//! The proxy sends the [`request::Request`] JSON — the message's
//! content blocks
//! — while a loop runs, one message at a time, and the program holds
//! the call open until the message's fate is known, then answers a
//! `2xx` with one [`Fate`]. Nothing times it out. A non-`2xx` is the
//! program declining to queue it; the proxy keeps the message,
//! offers nothing more to this loop, and starts the next loop on
//! what it holds once this one ends.

mod fate;
pub mod request;

pub use fate::*;
