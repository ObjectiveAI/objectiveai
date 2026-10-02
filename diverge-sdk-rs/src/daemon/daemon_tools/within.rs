//! Any, or only what is named: one side of a tagging tool's reach.

use serde::{Deserialize, Serialize};

/// How far one side of a tagging tool reaches: every thing of the
/// caller's, or only what `T` names — a filter for which agents,
/// tools or templates, a list of tags for which tags. Externally
/// tagged JSON: the string `"any"`, or `{"only":…}`. There is no
/// `disabled` here: the tool as a whole is held or not, by
/// [`Held`](super::Held), and each of its sides is always one of
/// these two.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Within<T> {
    /// Every one.
    Any,
    /// Only what `T` names. For tags, an empty list is no tag at
    /// all, and the tool acts with none.
    Only(T),
}
