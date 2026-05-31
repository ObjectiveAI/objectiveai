use crate::filesystem::tools::ManifestWithNameAndSource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shape: `{"type":"notification","value":{"tools":[...]}}`.
/// Emitted by `objectiveai tools list`. One entry per `.json`
/// manifest discovered in `<base_dir>/tools/`; failures during
/// discovery are silently dropped (see `Client::list_tools`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Tools")]
pub struct Tools {
    pub tools: Vec<ManifestWithNameAndSource>,
}

/// Wire shape: `{"type":"notification","value":{"tool": <manifest> | null}}`.
/// Emitted by `objectiveai tools get <name>`. The value is the
/// resolved `ManifestWithNameAndSource` when the manifest file exists
/// and parses, or JSON `null` when it doesn't (same silent-skip policy
/// as `list`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Tool")]
pub struct Tool {
    pub tool: Option<ManifestWithNameAndSource>,
}

/// One line of subprocess output from a tool spawned by
/// `objectiveai tools <name> <args…>`. Wire shape:
///
/// - stdout line: `{"line":"hello","stdout":true}`
/// - stderr line: `{"line":"oops","stderr":true}`
///
/// Exactly one of `stdout` / `stderr` is set per emission; the other
/// is omitted from the wire shape via `skip_serializing_if`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.ToolLine")]
pub struct ToolLine {
    pub line: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stdout: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub stderr: Option<bool>,
}
