//! The get response: the identity, none, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. The identity comes back as
//! [`Identity`](crate::daemon::endpoints::identities::list::server::response::Identity),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
