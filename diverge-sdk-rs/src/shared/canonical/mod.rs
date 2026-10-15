//! The one form a value is hashed in: compact JSON with every object
//! key sorted, at every depth.
//!
//! Every id that is a hash — a template's, the owner of a database
//! scope — is the SHA-256 of the value's canonical bytes, and nothing
//! else: the value as JSON with no whitespace, absent members
//! omitted, and the keys of EVERY object sorted bytewise, the
//! `arguments` and whatever maps lie inside them included, so that
//! two spellings of one value hash the same. `serde_json` runs with
//! `preserve_order` here, so a map keeps the order it was written in
//! and would hash differently per spelling without this. Array
//! element order is data and is kept. [`sort_object_keys`] is the
//! sort; [`bytes`] is the whole rule, for whoever hashes.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod bytes;
mod sort;

pub use bytes::*;
pub use sort::*;
