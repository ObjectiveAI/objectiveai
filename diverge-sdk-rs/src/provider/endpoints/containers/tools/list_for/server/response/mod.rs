//! What a provider sends back on a listing: a container, one at a
//! time, or a failure. See [`Frame`].

mod container;
mod frame;

pub use container::*;
pub use frame::*;
