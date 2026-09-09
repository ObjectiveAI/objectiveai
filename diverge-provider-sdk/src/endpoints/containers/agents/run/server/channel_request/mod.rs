//! The channels a provider opens on a caller during a run.
//!
//! Twenty, and the same twenty in both families: six the provider
//! asks on its own account, fourteen it relays from the container.
//! See [`Frame`].

mod frame;

pub use frame::*;
