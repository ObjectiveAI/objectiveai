//! The keys the issuer signs with.
//!
//! Every token is signed RS256 — RSASSA-PKCS1-v1_5 with SHA-256, the
//! algorithm OpenID Connect requires every provider to support — with
//! a 2048-bit RSA key. The rules:
//!
//! 1. A key is kept in `issuer.sqlite`, whole: the kid, the PEM
//!    (PKCS#1), and when it was made — see [`mint`].
//! 2. There is one key. At the start it is read — see
//!    [`Signing::load`]; with none, one is minted and recorded first.
//! 3. The key signs, and it is the one published in the key set. Were
//!    there more than one — a rotation stopped half way — the newest
//!    signs and every one is published.
//! 4. The key is read at the start, and held — with the key set made
//!    of it — as the issuer's [`Live`] keys.
//! 5. A rotation — see [`rotate`] — draws a new key, and records it
//!    and removes the old in one transaction; with signing out, it also
//!    revokes every refresh chain in that transaction. A running issuer
//!    rotates itself, on its loopback `POST /rotate` — see
//!    [`serve`](crate::serve) — and swaps its live keys at once: from
//!    then on only the new key signs and is published. A rotation made
//!    while no issuer runs takes effect at its start. Either way a
//!    token signed by the old key no longer verifies once a verifier's
//!    cache of the key set lapses, which
//!    [`jwks`](crate::endpoints::jwks) keeps to a minute.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod key;
mod live;
mod mint;
mod signing;

pub use error::*;
pub use key::*;
pub use live::*;
pub use mint::*;
pub use signing::*;
