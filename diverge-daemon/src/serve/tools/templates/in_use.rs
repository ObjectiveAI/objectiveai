//! Whether some tool was made from a template.

use crate::store::tools_templates::Record;

/// Whether some tool of the daemon's was made from the template.
/// No tool exists yet.
pub fn in_use(_: &Record) -> bool {
    false
}
