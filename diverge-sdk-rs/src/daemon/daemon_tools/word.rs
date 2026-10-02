//! The two words a reach is spelled with when it names nothing.

use serde::{Deserialize, Serialize};

/// `"disabled"` or `"any"`: what a [`Reach`](super::Reach), a
/// [`Held`](super::Held) or a [`Within`](super::Within) is on the
/// wire when it carries no value of its own. Each of the three reads
/// the words it admits and refuses the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Word {
    /// The tool is not held.
    Disabled,
    /// Every thing of the caller's.
    Any,
}
