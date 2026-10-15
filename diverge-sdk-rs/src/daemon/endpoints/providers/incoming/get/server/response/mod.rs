//! The get response: the credential, no such credential, forbidden, or
//! a failure.
//!
//! [`Frame`] is what a response frame holds. The credential comes back
//! as
//! [`Incoming`](crate::daemon::endpoints::providers::incoming::list::server::response::Incoming),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
