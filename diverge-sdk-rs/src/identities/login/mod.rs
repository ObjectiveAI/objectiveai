//! Signing an identity in: OAuth's device authorization grant, and
//! nothing else.
//!
//! [`login`](super::Identities::login) reads the issuer's discovery
//! document and asks it for a device code; the [`Pending`] it answers
//! carries the user code that is approved at the issuer, and
//! [`Pending::wait`] polls until it has been decided — see the
//! [rules](super), rule 6.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod device;
mod pending;

pub use pending::*;
