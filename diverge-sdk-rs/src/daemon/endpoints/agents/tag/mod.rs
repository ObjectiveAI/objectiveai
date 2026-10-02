//! Putting tags on an agent.
//!
//! One request, one answer. A client names an agent of its own and the
//! tags to put on it; the daemon answers that the tags are on it, that
//! no agent is the one named, or that it failed, and the scope finishes. A
//! tag the agent held already is held still, and is not a failure. A
//! tag is a string of the caller's choosing, compared and not read, as
//! a name is; what a agent holds is a set of them, which its list item
//! reports.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
