//! Whether some container mounts a resource.

use crate::store::resources::Record;

/// Whether some agent or tool of the daemon's mounts the resource —
/// one made from a template whose mounts name it. No container
/// exists yet.
pub fn in_use(_: &Record) -> bool {
    false
}
