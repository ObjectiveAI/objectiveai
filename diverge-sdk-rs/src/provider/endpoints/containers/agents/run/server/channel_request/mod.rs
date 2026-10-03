//! The channels a provider opens on a caller during a run.
//!
//! Twenty-eight, and the same twenty-eight in both families: seven
//! the provider asks on its own account, twenty-one it relays from
//! the container. See [`Frame`].

mod frame;

pub use frame::*;
