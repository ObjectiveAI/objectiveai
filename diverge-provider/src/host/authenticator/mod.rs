//! The provider's side of every handshake.
//!
//! [`ProviderAuthenticator`] is what the handshake asks, whichever end
//! dialled: the provider's own token, and whether a peer's token is
//! one the provider serves. The rules:
//!
//! 1. **Presenting.** The provider presents the token of its one
//!    `identity`, whatever issuer the peer's is from — see
//!    [`Own`](diverge_sdk::identities::Own).
//! 2. **Verifying.** There is no list of issuers. A peer's token is
//!    read for whom it claims to be, unsigned; one claiming an identity
//!    not in `accepted` is refused with nothing fetched; and only then
//!    is the token verified, against its own issuer's keys.
//! 3. **Admitting.** A verified peer is accepted exactly when its
//!    `(issuer, subject)` is in `accepted` and no connection holds it
//!    now. The rule is the same for a peer that dialled the provider
//!    and for one the provider dialled: the provider makes no
//!    distinction between the two.
//! 4. **Holding.** Accepting a peer takes its identity, [`Peers`], for
//!    the connection: a held identity is not a valid authentication,
//!    so a second connection under it is closed in the handshake. A
//!    handshake that fails after accepting gives it back, and so does
//!    the connection's end, through
//!    [`release`](ProviderAuthenticator::release). The provider's own
//!    identity is never held.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod authenticator;
mod peers;

pub use authenticator::*;
pub use peers::*;
