//! The channels a provider opens on a caller during a run.
//!
//! Twenty-seven: six the provider asks on its own account — the two
//! authorizations among them, which only a tool container is asked;
//! no deploy of tools, which only an agent container declares —
//! twenty-one it relays from the container. See [`Frame`].

mod frame;

pub use frame::*;
