//! The get response: the volume, no such volume, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds. The volume comes back as
//! [`Volume`](crate::daemon::endpoints::volumes::list::server::response::Volume),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
