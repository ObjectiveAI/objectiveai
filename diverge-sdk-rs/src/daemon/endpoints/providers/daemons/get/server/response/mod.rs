//! The get response: the daemon, no such daemon, forbidden, or a
//! failure. The daemon comes back as
//! [`Daemon`](crate::daemon::endpoints::providers::daemons::list::server::response::Daemon),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
