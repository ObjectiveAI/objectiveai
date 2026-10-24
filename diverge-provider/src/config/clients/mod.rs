//! The `clients` section of `provider.yaml`: the peers the provider
//! dials.
//!
//! Each [`Client`] is an address and the issuer the provider presents
//! as to it — its own identity there, from the document's
//! `identities`. The peer the provider reaches is judged by the same
//! `accepted` list as a peer that dials the provider: the provider
//! makes no distinction between the two.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod client;

pub use client::*;
