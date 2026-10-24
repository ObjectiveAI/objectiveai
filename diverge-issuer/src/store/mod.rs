//! `issuer.sqlite`: what the issuer knows.
//!
//! One SQLite file in the root, opened through sqlx by the issuer and
//! by the CLI alike — see [`Store::open`]. It holds the identities a
//! person signs in as, the clients that ask for tokens, the signing
//! keys' ids, and every code and refresh token issued and not yet
//! spent. It holds no secret: a password is an argon2id hash, and a
//! client secret, a code or a refresh token is a SHA-256. The signing
//! keys themselves are in the root's keyring.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod schema;
mod store;
mod write;

pub mod clients;
pub mod codes;
pub mod devices;
pub mod identities;
pub mod keys;
pub mod names;
pub mod refresh;

pub use error::*;
pub use store::*;
