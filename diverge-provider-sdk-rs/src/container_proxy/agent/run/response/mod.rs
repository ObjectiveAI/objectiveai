//! The answer: the loop's chunks, or why there are none. The chunks
//! themselves are defined where the provider protocol reads them,
//! [`agents::run::server::response`](crate::endpoints::containers::agents::run::server::response);
//! [`Frame`] is this path's envelope around one.

mod frame;

pub use frame::*;
