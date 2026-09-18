//! The channels a provider opens on a caller during a run.
//!
//! Twenty-four, and the same twenty-four in both families: five the
//! provider asks on its own account, nineteen it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
