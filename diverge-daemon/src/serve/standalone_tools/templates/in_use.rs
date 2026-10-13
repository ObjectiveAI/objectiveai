//! Whether some tool was made from a template.

use std::collections::HashSet;

use crate::store::tools_templates::Record;

/// Whether some created tool of the daemon's was made from the
/// template, given the ids of every template one was —
/// [`in_use::tools_templates`](crate::store::in_use::tools_templates),
/// loaded once per request.
pub fn in_use(held: &HashSet<String>, record: &Record) -> bool {
    held.contains(&record.id)
}
