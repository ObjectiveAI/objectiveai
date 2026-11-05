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
//! | [`signing`] | the RSA keys every token is signed with, kept in `issuer.sqlite`, and their rotation |
//! | [`tokens`] | the access token, the ID token, and the opaque secrets |
//! | [`endpoints`] | the routes, one file each |
//! | [`store`] | `issuer.sqlite`: the identities, the signing keys, the device codes and the refresh tokens |
//! | [`serve`] | the process: the root's lock, the port, the stop |
//!
//! The issuer serves one app, [`CLIENT_ID`](endpoints::CLIENT_ID): a
//! public client every Diverge program signs in as, by the device
//! authorization grant, kept signed in by rotating refresh tokens.
//! There is no client registry, no browser redirect, no client secret,
//! and no sign-in: there are no names and no passwords. A login names
//! the subject it asks for, as `login_hint`, and the issuer's operator,
//! on its host, approves or denies the code — the issuer is for local
//! use, and whoever runs it decides who exists. The binary serves and
//! does nothing else: the codes are decided, the identities listed and
//! removed, the keys rotated and the issuer started and stopped by the
//! Diverge CLI.
//!
//! The issuer holds `<root>/issuer.lock` for its life, as the daemon
//! and the provider hold theirs.

pub mod config;
pub mod endpoints;
pub mod serve;
pub mod signing;
pub mod store;
pub mod tokens;

mod clock;

pub use clock::*;

/// The lock file the issuer holds in the root for its life.
pub const LOCK_FILE: &str = "issuer.lock";
