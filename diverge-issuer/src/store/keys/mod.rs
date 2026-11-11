//! The signing keys' records.
//!
//! A record is a signing key: its key id, the `kid` every token signed
//! with it carries, and the key itself, as a PEM. There is one: a
//! rotation records the new key and removes the old.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod all;
mod keys;
mod record;

pub use keys::*;
pub use record::*;
