//! Whether some agent was made from a template.

use std::collections::HashSet;

use crate::store::agents_templates::Record;

/// Whether some agent of the daemon's was made from the template,
/// given the ids of every template one was —
/// [`in_use::agents_templates`](crate::store::in_use::agents_templates),
/// loaded once per request.
pub fn in_use(held: &HashSet<String>, record: &Record) -> bool {
    held.contains(&record.id)
}
