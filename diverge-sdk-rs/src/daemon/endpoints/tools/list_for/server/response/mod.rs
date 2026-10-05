//! The list_for response: the containers the provider tells of, one
//! each, no such provider, forbidden, or a failure.
//!
//! [`Frame`] is what a response frame holds. A container comes back as
//! the provider protocol's own
//! [`Container`](crate::provider::endpoints::containers::tools::list_for::server::response::Container),
//! relayed as it came.

mod frame;

pub use frame::*;
