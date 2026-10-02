//! The get response: the template, no such template, or a failure.
//!
//! [`Frame`] is what a response frame holds. The template comes back as
//! [`Listed`](crate::daemon::endpoints::tools::templates::list::server::response::Listed),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
