//! Whether a container holds a tagging tool, and if so how far each
//! side of it reaches.

use serde::{Deserialize, Serialize};

/// The reach of a tool that tags or untags, as a member of
/// [`DaemonTools`](super::DaemonTools) states it: not held, or held
/// with `T` saying which things and which tags, each side a
/// [`Within`](super::Within) of its own. There is no `any` at this
/// level, because "any agent with any tag" is `only` with both sides
/// `any`, and one word for it would hide which side is open.
/// Externally tagged JSON: the string `"disabled"`, or `{"only":…}`.
/// `Disabled` is the default, and what a member left out decodes as.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Held<T> {
    /// The container does not hold the tool.
    #[default]
    Disabled,
    /// The container holds the tool, reaching what `T` says on each
    /// side.
    Only(T),
}
