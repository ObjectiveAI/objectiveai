//! The list response: an event added or changed, the word that the
//! list is whole, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds — one
//! [`Event`](crate::daemon::endpoints::events::Event) as it comes to
//! be listed or its count changes, the word, forbidden, or a failure.
//! What one event is, is the family's: see
//! [`events`](crate::daemon::endpoints::events).

mod frame;

pub use frame::*;
