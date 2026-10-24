//! The provider's side of every handshake.
//!
//! [`ProviderAuthenticator`] is what the handshake asks, whichever end
//! dialled: the provider's own token at an issuer, and whether a peer's
//! token is one the provider serves. The rules:
//!
//! 1. **Presenting.** The provider presents, at an issuer, the token of
//!    the subject `identities` names there — see
//!    [`Own`](diverge_sdk::identities::Own).
//! 2. **Verifying.** A peer's token is verified against its issuer's
//!    keys, trusting exactly the issuers `identities` names.
//! 3. **Admitting.** A verified peer is served exactly when its
//!    `(issuer, subject)` is in `accepted`. The rule is the same for a
//!    peer that dialled the provider and for one the provider dialled:
//!    the provider makes no distinction between the two.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod authenticator;

pub use authenticator::*;
