//! The channels a provider opens on a caller during a run.
//!
//! Twenty-six: five the provider asks on its own account, twenty-one
//! it relays from the container. The tools family has two more, the
//! authorizations, which an agent container is never asked. See
//! [`Frame`].

mod frame;

pub use frame::*;
