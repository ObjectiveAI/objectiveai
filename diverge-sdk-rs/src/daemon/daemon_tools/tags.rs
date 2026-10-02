//! Which tags a tagging tool may put on, or take off.

use serde::{Deserialize, Serialize};

/// The tags a tool that tags, or untags, may act with: any tag at
/// all, or only those named. Externally tagged JSON: the string
/// `"any"`, or `{"only":[…]}` with the tags, one by one.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tags {
    /// Any tag: the tool puts on, or takes off, whatever it names.
    Any,
    /// Only these: a request naming a tag outside them is refused,
    /// and nothing changes. Empty, the tool may act with no tag at
    /// all.
    Only(Vec<String>),
}
