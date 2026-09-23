//! A provider, as the daemon knows one.
//!
//! The provider protocol carries no provider identity on its wire —
//! no name, no key, no address type; a provider says which revision
//! of the specification it speaks and nothing else about itself. Who
//! a provider is is a fact of the CONNECTION, and there are two ways
//! a daemon and a provider come to be connected: the daemon dialled
//! the provider, or the provider dialled the daemon and presented a
//! credential. [`Identity`] is one or the other. [`Provider`] holds
//! it, and is the shape every place that names a provider flattens
//! in, so that what the daemon comes to know later has somewhere to
//! go.

mod identity;
mod provider;

pub use identity::*;
pub use provider::*;
