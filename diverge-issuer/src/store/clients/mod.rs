//! The clients that ask the issuer for tokens.
//!
//! A client is an id, a [`Kind`] — public, with no secret, or
//! confidential, with one — the [`Grant`]s it may use, the audiences
//! its access tokens may be for, and, for the authorization code, the
//! redirect URIs it may be sent back to. It is added and removed by the
//! CLI, on the issuer's own host; the issuer reads it at every request.
//! Removing one removes every code and refresh token issued to it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod client;
mod clients;
mod grant;
mod kind;
mod list;
mod load;
mod new;
mod remove;

pub use client::*;
pub use clients::*;
pub use grant::*;
pub use kind::*;
pub use new::*;
