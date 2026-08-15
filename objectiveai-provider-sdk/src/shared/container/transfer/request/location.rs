//! One end of a transfer.

use serde::{Deserialize, Serialize};

/// A file, in a container.
///
/// One type used twice rather than four fields named in pairs, because
/// the two ends of a transfer are the same KIND of thing and a shape
/// that says so cannot be got wrong in one place and right in the
/// other.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Location {
    /// Which container.
    ///
    /// An [`Id`](crate::endpoints::containers::create::server::response::Frame::Id)
    /// from a creation. Both ends name one, so neither is taken from
    /// the scope this request arrives in — a transfer is not limited
    /// to the container a caller happens to be attached to, and the
    /// scope is what says a caller may ask rather than what it may ask
    /// about.
    pub container: String,
    /// Where in it, as path components from that container's root.
    ///
    /// The same meaning of "path" as everywhere else in this API, and
    /// the same frame of reference a
    /// [`filetree`](crate::shared::filetree) stream uses.
    pub path: Vec<String>,
}
