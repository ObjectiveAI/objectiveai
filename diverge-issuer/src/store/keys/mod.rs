//! The signing keys' records.
//!
//! A record is a signing key: its key id, the `kid` every token signed
//! with it carries, and the key itself, as a PEM. A record retired is
//! one whose key no longer signs and is no longer published, from the
//! issuer's next start.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod keys;
mod list;
mod record;
mod retire;
mod unretired;

pub use keys::*;
pub use record::*;
