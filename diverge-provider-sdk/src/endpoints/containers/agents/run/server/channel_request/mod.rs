//! The channels a provider opens on a caller during a run.
//!
//! Twenty-four, and the same twenty-four in both families: six the
//! provider asks on its own account, eighteen it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
