//! The channels a server opens on a caller during a creation.
//!
//! Two, and they have nothing to do with each other beyond both being
//! things a provider needs from the caller mid-scope — see [`Frame`].

mod frame;

pub use frame::*;
