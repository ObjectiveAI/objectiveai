//! The channels a provider opens on a caller during a run.
//!
//! Sixteen: five the provider asks on its own account — the
//! registration among them, carrying no dependencies, which only an
//! agent container declares — eleven it relays from the container.
//! See [`Frame`].

mod frame;

pub use frame::*;
