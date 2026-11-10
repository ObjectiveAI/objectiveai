//! `POST /enqueue`: the program's one slot, filled.
//!
//! The program holds at most ONE waiting message. The proxy sends the
//! [`request::Request`] JSON — every message it holds, in enqueue
//! order, each under its key — while a loop runs, and the program
//! holds the whole array as that one message: taken at a seam as ONE
//! user turn, its parts each under its own key. The call is held open
//! until the slot's fate is known, then answered `2xx` with one
//! [`Fate`], which is every message's in it. Nothing times it out.
//!
//! A second `/enqueue` while the slot is full is refused,
//! `409 {"kind":"occupied"}`; the proxy never sends one, because it
//! offers nothing more until the first is answered. Any other
//! non-`2xx` is the program declining the batch; the proxy keeps the
//! messages, offers nothing more to this loop, and starts the next
//! loop on what it holds once this one ends — but a `4xx` is the
//! batch refused, and every message in it is answered with the error.

mod fate;
pub mod request;

pub use fate::*;
