//! Putting tags on an identity.
//!
//! One request, one answer. A client names an identity and the tags to put
//! on it; the daemon answers that the tags are on it, that no identity on the
//! host is the one named, forbidden, or that it failed, and the scope finishes. A
//! tag the identity held already is held still, and is not a failure. A tag
//! is a string of the caller's choosing, compared and not read, as a
//! name is; an identity's tags are the daemon's, kept in its database, since an identity is a file
//! on the host and no record; what an identity holds is a set of them, which its list item
//! reports.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
