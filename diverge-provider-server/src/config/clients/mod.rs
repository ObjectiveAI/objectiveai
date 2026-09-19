//! The `clients` section of `config.yaml`.
//!
//! The peers the provider dials, and what it presents to each: the
//! [`Unbrokered`] clients, each an address, a key and an identity.
//! [`Clients`] holds them. A brokered mode is coming, and it will be
//! a second list beside this one, because which mode a peer is
//! dialled in is a fact of the configuration and not of the dial.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod clients;
mod unbrokered;

pub use clients::*;
pub use unbrokered::*;
