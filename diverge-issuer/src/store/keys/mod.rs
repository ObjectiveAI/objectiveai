//! The signing keys' records.
//!
//! A record names a signing key by its key id, the `kid` every token
//! signed with it carries, and by the keyring entry its PEM is kept
//! in. The key itself is never in the database. A record retired is
//! one whose key no longer signs and is no longer published.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod keys;
mod record;
mod unretired;

pub use keys::*;
pub use record::*;
