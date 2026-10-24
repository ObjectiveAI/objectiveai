//! `issuer.sqlite`: what the issuer knows.
//!
//! One SQLite file in the root, opened through sqlx by the issuer and
//! by the CLI alike — see [`Store::open`]. It holds the identities a
//! person signs in as, the signing keys, and every device code and
//! refresh token issued and not yet spent. A password is an argon2id
//! hash, and a device code or a refresh token a SHA-256; a signing
//! key is kept whole, since the issuer signs with it, which is why the
//! file is the owner's alone.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod schema;
mod store;
mod write;

pub mod devices;
pub mod identities;
pub mod keys;
pub mod names;
pub mod refresh;

pub use error::*;
pub use store::*;
