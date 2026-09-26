//! The channels a provider opens on a caller during a run.
//!
//! Twenty-seven, and the same twenty-seven in both families: six the
//! provider asks on its own account, twenty-one it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
