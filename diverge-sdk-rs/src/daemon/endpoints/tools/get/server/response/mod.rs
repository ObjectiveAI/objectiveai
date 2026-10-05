//! The get response: the tool, no such tool, or a failure.
//!
//! [`Frame`] is what a response frame holds. The tool comes back as
//! [`Tool`](crate::daemon::endpoints::tools::list::server::response::Tool),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
