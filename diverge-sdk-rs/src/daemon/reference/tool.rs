//! One tool of the caller's, named either way.

use serde::{Deserialize, Serialize};

/// One tool of the caller's: by its name, or by its template and
/// its index. See [`reference`](super) for which names what. Untagged
/// JSON, one object either way; an object with members of both
/// variants does not decode. A connected tool is reached by its name alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Tool {
    /// By name: `{"name":…}`.
    Name {
        /// The tool's name, as its create or its connect gave it.
        name: String,
    },
    /// By template and index: `{"template":…,"index":…}`.
    TemplateIndex {
        /// The template the tool was made from, by id.
        template: String,
        /// The tool's number among all tools of the caller's
        /// ever made from that template, as its list item carries it.
        index: u64,
    },
}
