//! `POST /token`: the token endpoint.
//!
//! A request is a form, as RFC 6749 §3.2 requires. The client is
//! checked first — the issuer's one app, see [`client`](fn@super::client)
//! — and then its `grant_type` names one of two grants:
//!
//! | `grant_type` | is | answers |
//! |--------------|----|---------|
//! | `urn:ietf:params:oauth:grant-type:device_code` | a device code, polled — see [`device_code`] | an access token, a refresh token, and an ID token for `openid`, once a person approves it |
//! | `refresh_token` | a refresh token, spent for the next — see [`refresh_token`] | an access token, a new refresh token, and an ID token for `openid` |
//!
//! A `grant_type` that is none of these is `unsupported_grant_type`.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod answer;
mod device_code;
mod form;
mod refresh_token;
mod token;

pub use answer::*;
pub use device_code::*;
pub use form::*;
pub use refresh_token::*;
pub use token::*;
