//! Whether some container mounts a resource.

use std::collections::HashSet;

use crate::store::resources::Record;

/// Whether some agent or tool of the daemon's mounts the resource —
/// one made from a template whose mounts name it — given the ids of
/// every resource one does,
/// [`in_use::resources`](crate::store::in_use::resources), loaded
/// once per request.
pub fn in_use(held: &HashSet<String>, record: &Record) -> bool {
    held.contains(&record.id)
}
