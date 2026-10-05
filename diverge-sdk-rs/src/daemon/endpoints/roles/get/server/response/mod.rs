//! The get response: the role, no such role, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. The role comes back as
//! [`Role`](crate::daemon::endpoints::roles::list::server::response::Role),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
