//! The get response: the judge, no such judge, or a failure. The judge
//! comes back as
//! [`Incoming`](crate::daemon::endpoints::providers::incoming::list::server::response::Incoming),
//! the very shape a list reports it in, defined beside the list and not
//! repeated here.

mod frame;

pub use frame::*;
