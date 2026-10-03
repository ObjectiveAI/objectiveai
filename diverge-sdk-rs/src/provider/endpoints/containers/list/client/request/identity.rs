//! Whose containers: an identity, named the way it was authorized.

use serde::{Deserialize, Serialize};

/// The identity whose containers a listing asks for, by the mode it
/// authenticates in. JSON-tagged by `kind`.
///
/// One variant today, and a tag all the same, for the reason
/// [`Auth`](crate::wire::frame::auth::Auth) ships a mode byte for one
/// mode: a brokered identity is a different name — a broker's word
/// for a peer — and its variant takes the tag `brokered` when it is
/// defined, without breaking the wire.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Identity {
    /// An identity the provider knows on its own: the string an
    /// unbrokered authorization established, as the provider's
    /// [`UnbrokeredAuthorizer`](crate::wire::server::unbrokered_authorizer::UnbrokeredAuthorizer)
    /// judged it. Compared, not read.
    Unbrokered {
        /// The identity.
        identity: String,
    },
}
