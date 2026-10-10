//! Taking tags off a volume.
//!
//! One request, one answer. A client names a volume and the tags to take
//! off it; the daemon answers that the tags are off it, that no volume
//! has the reference, forbidden, or that it failed, and the scope finishes.
//! A tag the volume did not hold is not a failure. A tag is a string of
//! the caller's choosing, compared and not read, as a name is; what a
//! volume holds is a set of them, which its list item reports.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
