//! The channels a provider opens on a caller during a run.
//!
//! Eighteen, and the same eighteen in both families: six the
//! provider asks on its own account, twelve it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
