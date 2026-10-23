//! The keys `provider.yaml` names, read from the host's keyring once,
//! at start.
//!
//! The document holds no key. Every place it would — a key of
//! `auth.unbrokered`, a key of `clients.unbrokered`, a registry's
//! password — names an entry of the
//! [`keyring`](diverge_sdk::keyring) at the root instead, and
//! [`Keys::resolve`] reads every one before the provider starts: an
//! entry the keyring does not hold stops the start, naming it. A key
//! put in the keyring later is read at the next start; the provider
//! reads `provider.yaml` once, too.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod keys;

pub use error::*;
pub use keys::*;
