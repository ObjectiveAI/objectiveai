//! Putting tags on a daemon.
//!
//! One request, one answer. A client names a daemon and the tags to put
//! on it; the daemon answers that the tags are on it, that no record has the
//! name, forbidden, or that it failed, and the scope finishes. A
//! tag the record held already is held still, and is not a failure. A tag
//! is a string of the caller's choosing, compared and not read, as a
//! name is; a daemon's tags are the daemon's, kept on the record; what a record holds is a set of them, which its list item
//! reports.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
