//! Whether some agent was made from a template.

use crate::store::agents_templates::Record;

/// Whether some agent of the daemon's was made from the template.
/// No agent exists yet.
pub fn in_use(_: &Record) -> bool {
    false
}
