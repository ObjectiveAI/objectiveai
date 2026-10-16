//! The channels a provider opens on a caller during a run.
//!
//! Fifteen: four the provider asks on its own account — no deploy of
//! dependencies, which only an agent container declares — eleven it
//! relays from the container. See [`Frame`].

mod frame;

pub use frame::*;
