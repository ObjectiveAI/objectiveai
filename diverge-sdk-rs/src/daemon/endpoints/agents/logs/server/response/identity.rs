//! Who a provider is.

/// A provider's identity: the `(issuer, subject)` its token
/// established in the handshake, whichever end dialled — see
/// [`shared::Identity`](crate::shared::Identity).
pub type Identity = crate::shared::Identity;
