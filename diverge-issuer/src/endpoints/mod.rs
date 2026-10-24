//! The issuer's endpoints, one file per route.
//!
//! | route | standard | is |
//! |-------|----------|----|
//! | `GET /.well-known/openid-configuration` | OpenID Connect Discovery | the provider's metadata — see [`discovery`] |
//! | `GET /jwks` | RFC 7517 | the key set — see [`jwks`] |
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod discovery;
mod jwks;
mod paths;

pub use discovery::*;
pub use jwks::*;
pub use paths::*;
