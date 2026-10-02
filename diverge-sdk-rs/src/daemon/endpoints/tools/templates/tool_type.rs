//! The one value a tool template's `type` takes.

use serde::{Deserialize, Serialize};

/// `"tool"`, and nothing else: the `type` of a tool template, which
/// is what tells it from an agent template with the same image,
/// limits, mounts and arguments. A template whose `type` is anything
/// else does not decode as a tool template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    /// The only value.
    #[default]
    Tool,
}
