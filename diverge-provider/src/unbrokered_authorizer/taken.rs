//! What one connection's authorizer took, for the connection to give
//! back.

use std::sync::Arc;

use tokio::sync::Mutex;

/// The identity and the credential digest a connection holds in the
/// provider's [`Peers`](crate::serve::Peers), once its credential was
/// accepted. The authorizer writes it; the connection reads it when
/// the connection is over and gives both back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The identity admitted.
    pub identity: String,
    /// The credential's digest.
    pub credential: [u8; 32],
}

/// Where a connection's authorizer leaves what it took: empty until a
/// credential is accepted, and empty for a connection whose never is.
pub type Slot = Arc<Mutex<Option<Taken>>>;
