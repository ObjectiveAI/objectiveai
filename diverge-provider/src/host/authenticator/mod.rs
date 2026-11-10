//! The provider's side of every handshake.
//!
//! [`ProviderAuthenticator`] holds the provider's side, and a
//! [`Judging`] of it is what one handshake asks: the provider's own
//! token, and whether a peer's token is one the provider serves on
//! that connection. The rules:
//!
//! 1. **Presenting.** The provider presents the token of its one
//!    `identity`, whatever issuer the peer's is from — see
//!    [`Own`](diverge_sdk::identities::Own).
//! 2. **Verifying.** There is no list of issuers. A peer's token is
//!    read for whom it claims to be, unsigned; one claiming an identity
//!    rule 3 does not admit is refused with nothing fetched; and only
//!    then is the token verified, against its own issuer's keys.
//! 3. **Admitting.** A peer that dialled the provider is admitted
//!    exactly when its `(issuer, subject)` is in `incoming`; a peer
//!    the provider dialled, exactly when it is the identity that
//!    `clients` entry names. Neither list has a say over the other's
//!    connections. Either way, no connection may hold the identity
//!    now.
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
