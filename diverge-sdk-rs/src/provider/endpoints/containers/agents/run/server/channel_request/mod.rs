//! The channels a provider opens on a caller during a run.
//!
//! Twenty: five the provider asks on its own account, fifteen it
//! relays from the container. The tools family has the same five of
//! its own — its registration carries no dependencies, which a tool
//! container is never asked about — and eleven relayed. See
//! [`Frame`].

mod frame;

pub use frame::*;
