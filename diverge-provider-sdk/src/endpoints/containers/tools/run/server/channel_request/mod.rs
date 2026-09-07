//! The channels a provider opens on a caller during a run.
//!
//! Seventeen, and the same seventeen in both families: five the
//! provider asks on its own account, twelve it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
