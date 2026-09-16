//! The channels a provider opens on a caller during a run.
//!
//! Twenty-three, and the same twenty-three in both families: four the
//! provider asks on its own account, nineteen it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
