//! The channels a caller opens on a provider.
//!
//! Three. One calls the plugin, one ends the scope that opened it, and
//! one collects the writes of a database connection the provider
//! opened — see [`Frame`].
//!
//! That third one is the odd member, and [`Postgres`] says why: it
//! opens outward not because of where anything is, but because a
//! stream needs a responder to end it.

mod frame;
mod postgres;

pub use frame::*;
pub use postgres::*;
