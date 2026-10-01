//! Taking tags off a tool.
//!
//! One request, one answer. A client names a tool of its own and the
//! tags to take off it; the daemon answers that the tags are off it,
//! that no tool has that name, or that it failed, and the scope
//! finishes. A tag the tool did not hold is not held still, and is not
//! a failure. A tag is a string of the caller's choosing, compared and
//! not read, as a name is; what a tool holds is a set of them, which
//! its list item reports.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer
//! is in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
