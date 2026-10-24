//! The keys the issuer signs with.
//!
//! Every token is signed RS256 — RSASSA-PKCS1-v1_5 with SHA-256, the
//! algorithm OpenID Connect requires every provider to support — with
//! a 2048-bit RSA key. The rules:
//!
//! 1. A key is kept in `issuer.sqlite`, whole: the kid, the PEM
//!    (PKCS#1), and when it was made — see [`mint`].
//! 2. At the start, every unretired key is read — see
//!    [`Signing::load`]. With none, one is minted and recorded first.
//! 3. The newest unretired key signs; every unretired key is
//!    published in the key set, so a token signed by an older key
//!    still verifies.
//! 4. The keys are read once, at the start: a key added or retired
//!    while the issuer runs is seen at its next start.
//! 5. A rotation — see [`rotate`] — mints a new key and retires every
//!    other; with signing out, it also revokes every refresh chain. It
//!    takes effect at the issuer's next start: from then on only the
//!    new key signs and is published, so a token signed by an old key
//!    no longer verifies once a verifier's cache of the key set lapses,
//!    which [`jwks`](crate::endpoints::jwks) keeps to a minute.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod key;
mod mint;
mod signing;

pub use error::*;
pub use key::*;
pub use mint::*;
pub use signing::*;
