//! `POST /token`: the token endpoint.
//!
//! A request is a form, as RFC 6749 §3.2 requires. The client is
//! authenticated first — see [`authenticate`](super::authenticate) —
//! and then its `grant_type` names one of four grants, each the
//! client must have:
//!
//! | `grant_type` | is | answers |
//! |--------------|----|---------|
//! | `authorization_code` | a code from the sign-in page, its redirect URI again, and its PKCE verifier — see [`authorization_code`] | an access token; an ID token for `openid`; a refresh token for a client with `refresh_token` |
//! | `urn:ietf:params:oauth:grant-type:device_code` | a device code, polled — see [`device_code`] | as the authorization code, once a person approves it |
//! | `refresh_token` | a refresh token, spent for the next — see [`refresh_token`] | an access token, a new refresh token, and an ID token for `openid` |
//! | `client_credentials` | the confidential client's secret alone — see [`client_credentials`] | an access token whose subject is the client; no refresh token, no ID token |
//!
//! A `grant_type` that is none of these is `unsupported_grant_type`;
//! a grant the client does not have is `unauthorized_client`.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod answer;
mod authorization_code;
mod client_credentials;
mod device_code;
mod form;
mod refresh_token;
mod token;

pub use answer::*;
pub use authorization_code::*;
pub use client_credentials::*;
pub use device_code::*;
pub use form::*;
pub use refresh_token::*;
pub use token::*;
