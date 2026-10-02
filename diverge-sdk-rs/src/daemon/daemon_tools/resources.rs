//! Which resources a resources tool may list, or delete.

use serde::{Deserialize, Serialize};

/// The resources a tool that lists, or deletes, resources reaches:
/// any resource of the caller's, or only those named by id. Externally
/// tagged JSON: the string `"any"`, or `{"only":[…]}` with the ids,
/// one by one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resources {
    /// Any resource of the caller's.
    Any,
    /// Only these, by id — the hash an
    /// [`upload`](crate::daemon::endpoints::resources::upload)
    /// answered. A request naming one outside them is refused, and
    /// nothing changes; a list answers none outside them. Empty, the
    /// tool reaches no resource at all.
    Only(Vec<String>),
}
