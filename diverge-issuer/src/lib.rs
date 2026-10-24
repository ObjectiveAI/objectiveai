//! The Diverge issuer: an OpenID Connect provider.
//!
//! The issuer signs the tokens a Diverge server admits a caller by. It
//! speaks OAuth 2.0 and OpenID Connect over plain HTTP, as the
//! standards define them, so a client written to the standards gets
//! tokens from it as from any other provider, and a server that
//! verifies tokens the standard way verifies its tokens as any other
//! provider's.
//!
//! | module | is |
//! |--------|----|
//! | [`config`] | `issuer.yaml`, and the root it is found in |
//! | [`password`] | passwords, as argon2id hashes |
//! | [`signing`] | the RSA keys every token is signed with, kept in the root's keyring |
//! | [`tokens`] | the access token, the ID token, and the opaque secrets |
//! | [`endpoints`] | the routes, one file each |
//! | [`store`] | `issuer.sqlite`: the identities, the clients, the codes and the refresh tokens, every secret in it a hash |
//! | [`serve`] | the process: the root's lock, the port, the stop |
//!
//! The issuer holds `<root>/issuer.lock` for its life, as the daemon
//! and the provider hold theirs.

pub mod config;
pub mod endpoints;
pub mod password;
pub mod serve;
pub mod signing;
pub mod store;
pub mod tokens;

mod clock;

pub use clock::*;

/// The lock file the issuer holds in the root for its life.
pub const LOCK_FILE: &str = "issuer.lock";
