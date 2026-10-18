//! The get response: the account, no such account, forbidden, or a
//! failure.
//!
//! [`Frame`] is what a response frame holds. The account comes back as
//! [`Account`](crate::daemon::endpoints::accounts::list::server::response::Account),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
