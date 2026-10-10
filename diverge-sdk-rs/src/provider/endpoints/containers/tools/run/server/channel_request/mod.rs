//! The channels a provider opens on a caller during a run.
//!
//! Twenty-four: four the provider asks on its own account — no deploy
//! of dependencies, which only an agent container declares — twenty
//! it relays from the container. See [`Frame`].

mod frame;

pub use frame::*;
