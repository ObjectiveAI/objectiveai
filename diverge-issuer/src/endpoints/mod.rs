//! The issuer's endpoints, one file per route.
//!
//! | route | standard | is |
//! |-------|----------|----|
//! | `GET /.well-known/openid-configuration` | OpenID Connect Discovery | the provider's metadata — see [`discovery`] |
//! | `GET /jwks` | RFC 7517 | the key set — see [`jwks`] |
//! | `GET`, `POST /authorize` | OpenID Connect Discovery | named, and refused: the one app signs in by device code — see [`authorize`] |
//! | `POST /device_authorization` | RFC 8628 | a device code and its user code — see [`device_authorization`] |
//! | `GET`, `POST /device` | RFC 8628 | the page a user code is approved on — see [`device_page`] and [`device_decide`] |
//! | `POST /token` | RFC 6749 | the tokens, by two grants — see [`token`](mod@token) |
//! | `POST /revoke` | RFC 7009 | a refresh token's chain revoked — see [`revoke`] |
//! | `GET`, `POST /userinfo` | OpenID Connect Core | the bearer token's subject — see [`userinfo`] |
//!
//! Every request is a form or a query, every answer JSON or, for the
//! device page, HTML; every error of the OAuth endpoints is
//! [`OAuthError`]'s shape. Every request is from the issuer's one app,
//! [`CLIENT_ID`], a public client — see [`client`](fn@client).
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod authorize;
mod client;
mod device;
mod device_authorization;
mod discovery;
mod error;
mod grant_scope;
mod jwks;
mod page;
mod paths;
mod revoke;
mod user_code;
mod userinfo;

pub mod token;

pub use authorize::*;
pub use client::*;
pub use device::*;
pub use device_authorization::*;
pub use discovery::*;
pub use error::*;
pub use grant_scope::*;
pub use jwks::*;
pub use page::*;
pub use paths::*;
pub use revoke::*;
pub use token::token;
pub use userinfo::*;
