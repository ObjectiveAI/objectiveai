//! Passwords, as argon2id hashes.
//!
//! A password is never kept: [`hash`] makes the PHC string the
//! database holds, and [`verify`] checks a password a person typed
//! against one. Both run argon2id with its default parameters, on the
//! blocking pool, since a hash is meant to be slow.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod hash;
mod verify;

pub use error::*;
pub use hash::*;
pub use verify::*;
