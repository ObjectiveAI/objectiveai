//! The word a caller names itself by.

use serde::{Deserialize, Serialize};

/// `"self"`, and nothing else: the one value a reference to the
/// caller itself takes. A reference's `Itself` variant wraps it, so
/// that the reference is the string `"self"` on the wire where its
/// other forms are objects, and nothing is mistaken for a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Itself {
    /// The only value.
    #[serde(rename = "self")]
    Itself,
}
