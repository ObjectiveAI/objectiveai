//! The channels a provider opens on a caller during a run.
//!
//! Twenty-five, and the same twenty-five in both families: six the
//! provider asks on its own account, nineteen it relays from the
//! container. See [`Frame`].

mod frame;

pub use frame::*;
