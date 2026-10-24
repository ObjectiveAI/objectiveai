//! The other end, verified.

/// The identity the other end's token established: its issuer and its
/// subject, as its [`Authenticator`](super::Authenticator) verified
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Peer {
    /// The token's `iss`.
    pub issuer: String,
    /// The token's `sub`.
    pub subject: String,
}
