//! Taking tags off a resource.
//!
//! One request, one answer. A client names a resource by its id and the
//! tags to take off it; the daemon answers that the tags are off it,
//! that no resource has the id, forbidden, or that it failed, and the
//! scope finishes. A tag the resource did not hold is not a failure. A
//! tag is a string of the caller's choosing, compared and not read;
//! what a resource holds is a set of them, which its list item reports,
//! outside the resource's hash.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in [`client`] — and the daemon answers, so the answer is
//! in [`server`]. Neither side holds both halves of the exchange.

pub mod client;
pub mod server;
