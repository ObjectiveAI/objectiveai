//! The channels a provider opens on a caller during a run.
//!
//! Twenty-five: five the provider asks on its own account, twenty it
//! relays from the container. The tools family has one fewer, the
//! dependencies, which a tool container is never asked. See
//! [`Frame`].

mod frame;

pub use frame::*;
