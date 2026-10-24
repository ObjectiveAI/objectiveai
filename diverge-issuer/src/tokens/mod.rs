//! The tokens the issuer signs.
//!
//! - The access token is an RFC 9068 JWT — see [`Access`] and
//!   [`access`]. It is what a client presents to a server, and what
//!   the server verifies by the key set alone.
//! - The ID token is OpenID Connect's — see [`id`] — issued beside an
//!   access token when the scope holds `openid`, for the client to
//!   learn who signed in.
//! - A refresh token, an authorization code and a device code are no
//!   JWT: each is 32 random bytes as hex, kept only as its SHA-256 —
//!   see [`mint`] and [`hash`].
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod access;
mod error;
mod id;
mod secret;
mod verify;

pub use access::*;
pub use error::*;
pub use id::*;
pub use secret::*;
pub use verify::*;
