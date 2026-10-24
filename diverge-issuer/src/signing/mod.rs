//! The keys the issuer signs with.
//!
//! Every token is signed RS256 — RSASSA-PKCS1-v1_5 with SHA-256, the
//! algorithm OpenID Connect requires every provider to support — with
//! a 2048-bit RSA key. The rules:
//!
//! 1. A key's PEM (PKCS#1) is kept in the root's keyring as
//!    `issuer/signing/<kid>`, and its record — the kid, the entry,
//!    when it was made — in `issuer.sqlite`. The database never holds
//!    the key.
//! 2. At the start, every unretired key is read — see
//!    [`Signing::load`]. With none, one is minted and recorded first.
//!    A record whose entry the keyring does not hold stops the start.
//! 3. The newest unretired key signs; every unretired key is
//!    published in the key set, so a token signed by an older key
//!    still verifies.
//! 4. The keys are read once, at the start: a key added or retired
//!    while the issuer runs is seen at its next start.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod key;
mod mint;
mod signing;

pub use error::*;
pub use key::*;
pub use signing::*;
