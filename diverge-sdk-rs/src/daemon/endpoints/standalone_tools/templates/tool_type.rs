//! What a tool template says of itself beside what every template
//! says: that it is one, and how its tools are kept.

use serde::{Deserialize, Serialize};

use super::Mode;

/// `"type":"tool"`, and the `mode` beside it: what tells a tool
/// template from an agent template with the same image, limits,
/// mounts and arguments, and how every tool made from it is kept —
/// see [`Mode`]. Flat among the template's members: a template whose
/// `type` is anything else does not decode as a tool template, and
/// one that says no `mode` is transient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename = "tool")]
pub struct ToolType {
    /// How a tool made from the template is kept running: see
    /// [`Mode`]. Absent is transient.
    #[serde(default)]
    pub mode: Mode,
}

/// A tool template: the daemon's one
/// [`Template`](crate::daemon::template::Template), typed
/// [`ToolType`].
pub type Template = crate::daemon::template::Template<ToolType>;
