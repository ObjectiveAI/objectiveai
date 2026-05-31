use crate::filesystem::plugins::ManifestWithNameAndSource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shape: `{"type":"notification","value":{"plugins":[...]}}`.
/// Emitted by `objectiveai plugins list`. One entry per `.json`
/// manifest discovered in `<base_dir>/plugins/`; failures during
/// discovery are silently dropped (see `Client::list_plugins`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Plugins")]
pub struct Plugins {
    pub plugins: Vec<ManifestWithNameAndSource>,
}

/// Wire shape: `{"type":"notification","value":{"plugin": <manifest> | null}}`.
/// Emitted by `objectiveai plugins get <name>`. The value is the
/// resolved `ManifestWithNameAndSource` when the manifest file exists
/// and parses, or JSON `null` when it doesn't (same silent-skip policy
/// as `list`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Plugin")]
pub struct Plugin {
    pub plugin: Option<ManifestWithNameAndSource>,
}

/// Wire shape: `{"type":"notification","value":{"installed": true|false}}`.
/// Emitted by `objectiveai plugins install`. `true` means the binary
/// landed at `<base_dir>/plugins/<repository>/plugin[.exe]`; `false`
/// means the manifest fetched OK but this platform isn't in its
/// `binaries` map (nothing to install, not an error).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Installed")]
pub struct Installed {
    pub installed: bool,
}

/// Wire shape: `{"type":"notification","value":{"kind":"mcp","url":"…"}}`.
/// Emitted by a running plugin to announce an MCP server URL it wants
/// the host to expose. The host routes this through the standard
/// plugin-notification pipeline and dials the URL the same way it
/// would for an entry in the plugin's manifest `mcp_servers` —
/// runtime announcements are functionally identical to manifest-time
/// declarations.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.Mcp")]
pub struct Mcp {
    pub url: String,
}
