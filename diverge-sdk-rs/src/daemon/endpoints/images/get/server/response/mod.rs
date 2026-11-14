//! The get response: the image, none, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. The image comes back as
//! [`Image`](crate::daemon::endpoints::images::list::server::response::Image),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
