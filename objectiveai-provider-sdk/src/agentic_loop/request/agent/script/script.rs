//! The script itself.

use serde::{Deserialize, Serialize};

/// Code run in place of a model.
///
/// Tagged by `type` rather than untagged, unlike most enums here: the
/// variants are distinguished by which language they carry, and a
/// second language would otherwise be told apart only by its field
/// name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Script {
    /// Python, run on the caller's embedded runtime.
    Python {
        /// The source, verbatim. Never normalized — whitespace is
        /// significant in Python, so the usual trimming would change
        /// what the code means.
        python: String,
    },
}
