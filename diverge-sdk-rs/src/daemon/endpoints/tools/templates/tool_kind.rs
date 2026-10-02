//! What is a tool template's own: its type, and nothing else.

use serde::{Deserialize, Serialize};

use super::ToolType;

/// The front of a tool template, flattened into its JSON: the
/// `type`, always `"tool"`. A tool holds none of the daemon's tools,
/// so there is nothing else here; `{"type":"tool"}`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolKind {
    /// `"tool"`. See [`ToolType`].
    pub r#type: ToolType,
}

/// A tool template: the daemon's one
/// [`Template`](crate::daemon::template::Template), led by
/// [`ToolKind`].
pub type Template = crate::daemon::template::Template<ToolKind>;
