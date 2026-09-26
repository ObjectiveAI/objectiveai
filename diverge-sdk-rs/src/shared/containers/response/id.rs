//! The container's id.

use serde::{Deserialize, Serialize};

/// What the provider decided to call this container.
///
/// An object rather than a bare string, so a provider that later has
/// something else to say about the container's identity has somewhere
/// to say it. A JSON string is a shape that can only ever be one
/// field.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Id {
    /// The id itself.
    ///
    /// Opaque, and the provider's to mint. A caller that wants to name
    /// this container anywhere else — a
    /// [`Connect`](crate::shared::containers::request::Connect) — has
    /// this and nothing else to name it with. It is a capability:
    /// holding it is what lets a connector ask, so it has to be
    /// unguessable, and nothing a caller could choose would be.
    ///
    /// Nothing here constrains its shape. It means nothing to anyone
    /// who was not given it, and nothing outside the provider that
    /// minted it.
    pub id: String,
}
