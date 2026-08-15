//! The channels a server opens on a caller during a run.
//!
//! Three, and they have nothing to do with each other beyond all
//! being things a provider needs from the caller mid-scope — see
//! [`Frame`].

mod authorize;
mod frame;

pub use authorize::*;
pub use frame::*;
