//! The get response: the provider, no such provider, forbidden, or a
//! failure. The provider comes back as
//! [`Provider`](crate::daemon::endpoints::providers::list::server::response::Provider),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
