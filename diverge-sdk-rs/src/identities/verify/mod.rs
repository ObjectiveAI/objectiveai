//! Verifying another end's token, against its issuer's keys, cached.
//!
//! [`KeySets`] is the cache every verification takes: each issuer's
//! key set, fetched once and held until it expires.
//! [`verify`](KeySets::verify) judges a token by it. The rules:
//!
//! 1. **Trust before fetch.** The token's `iss` is read from its
//!    payload with nothing checked, and the caller's `trusted` must
//!    accept it before anything is fetched, else
//!    [`VerifyError::Untrusted`]. No key set is fetched for an issuer
//!    the caller does not trust, so no token makes this end fetch from
//!    a server of the token's choosing.
//! 2. **The header.** Its `alg` is asymmetric — RS256, RS384, RS512,
//!    PS256, PS384, PS512, ES256, ES384 or EdDSA — else
//!    [`VerifyError::Algorithm`]: an issuer's public keys cannot check
//!    a shared secret, and `none` checks nothing. It names a `kid`,
//!    else [`VerifyError::Malformed`].
//! 3. **The lookup.** The issuer's entry is taken from the map — made
//!    empty where there is none — and the map's guard let go before
//!    any lock is awaited. Then:
//!    1. The entry's read lock is taken. An entry that is fresh and
//!       holds the `kid` is used, and nothing is fetched; many readers
//!       hold the read lock at once.
//!    2. Else the read lock is let go and the write lock taken.
//!    3. The entry is looked at again: another task may have refreshed
//!       it while this one waited. One that is fresh and holds the
//!       `kid` now is used, and the write lock let go.
//!    4. Else the key set is fetched, the entry replaced, and used. At
//!       most one fetch per issuer runs at once: the write lock's.
//! 4. **Fresh.** An entry is fresh until it expires: after the key
//!    set answer's `Cache-Control: max-age`, bounded to between a
//!    minute and a day, or else after the cache's own lifetime.
//! 5. **Rotation.** A fresh entry without the `kid` is an issuer that
//!    rotated its keys, and is fetched again as rule 3 does — unless
//!    it was fetched under a minute ago, so that a run of made-up
//!    `kid`s cannot make this end fetch on every token. A `kid` an
//!    entry still lacks is [`VerifyError::UnknownKey`].
//! 6. **The fetch.** The issuer's discovery document, which must name
//!    the issuer exactly and a `jwks_uri`; then that key set. A fetch
//!    that fails is [`VerifyError::Fetch`], and the entry is left as
//!    it was — an expired one is never used.
//! 7. **The check.** The signature is checked with the `kid`'s key.
//!    `iss` must be the issuer the keys are, `exp` must not have
//!    passed, give or take a minute, and `aud` must hold the
//!    `audience` asked, where one is. `exp`, `iss` and `sub` are
//!    required. Any failure is [`VerifyError::Invalid`].
//! 8. **The answer** is [`Verified`]: the token's issuer, subject and
//!    expiry.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod entry;
mod error;
mod fetch;
mod key_sets;
mod keys;
mod verified;
mod verify;

pub use error::*;
pub use key_sets::*;
pub use verified::*;
