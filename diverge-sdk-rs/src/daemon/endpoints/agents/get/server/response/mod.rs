//! The get response: the agent, no such agent, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. The agent comes back as
//! [`Agent`](crate::daemon::endpoints::agents::list::server::response::Agent),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
