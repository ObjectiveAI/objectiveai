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
    /// [`connect`](crate::endpoints::laboratories::connect), a
    /// [`transfer`](crate::shared::container::transfer)'s destination
    /// — has this and nothing else to name it with.
    ///
    /// Nothing here constrains its shape. It means nothing to anyone
    /// who was not given it, and nothing outside the provider that
    /// minted it.
    pub id: String,
}
