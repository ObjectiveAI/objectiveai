//! How far one of the daemon's tools reaches: not at all, everything,
//! or only what is named.

use serde::{Deserialize, Serialize};

/// The reach of one of the daemon's tools, as a member of
/// [`DaemonTools`](super::DaemonTools) states it. Externally tagged
/// JSON: the string `"disabled"`, the string `"any"`, or
/// `{"only":…}` carrying `T` — a filter, a pair of filters, a
/// filter with its tags, or a list of ids, as the member says.
/// `Disabled` is the default, and what a member left out decodes
/// as.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reach<T> {
    /// The container does not hold the tool.
    #[default]
    Disabled,
    /// The container holds the tool, and it reaches every thing of
    /// the caller's.
    Any,
    /// The container holds the tool, and it reaches only what `T`
    /// names.
    Only(T),
}
