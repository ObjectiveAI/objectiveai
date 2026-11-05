//! `POST /token`: the token endpoint.
//!
//! A request is a form, as RFC 6749 §3.2 requires. The client is
//! checked first — the issuer's one app, see [`client`](fn@super::client)
//! — and then its `grant_type` names one of two grants:
//!
//! | `grant_type` | is | answers |
//! |--------------|----|---------|
//! | `password` | a subject and its password, RFC 6749 §4.3 — see [`password`](fn@password) | an access token, a refresh token beginning a new chain, and an ID token for `openid` |
//! | `refresh_token` | a refresh token, spent for the next — see [`refresh_token`] | an access token, a new refresh token, and an ID token for `openid` |
//!
//! A `grant_type` that is none of these is `unsupported_grant_type`.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod answer;
mod form;
mod password;
mod refresh_token;
mod token;

pub use answer::*;
pub use form::*;
pub use password::*;
pub use refresh_token::*;
pub use token::*;
