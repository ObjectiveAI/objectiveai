//! The channels a caller opens on a provider.
//!
//! Two. One calls the plugin; the other ends the scope that opened it
//! — see [`Frame`].

mod frame;

pub use frame::*;
