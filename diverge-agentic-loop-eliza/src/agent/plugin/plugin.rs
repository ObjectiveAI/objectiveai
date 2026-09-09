//! One plugin the caller wants in the runtime.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// An elizaOS plugin the container installs and loads for the run —
/// any npm package that exports a `Plugin`, configured by its own
/// setting names, its secrets the vault's.
///
/// HOW THE HARNESS APPLIES IT: at the start of the run, every named
/// package the image's project does not already hold is installed
/// into it (`bun add --ignore-scripts`, the command elizaOS's own
/// installer runs), imported, and handed to the runtime's constructor
/// beside the image's plugins; the version the registry resolved is
/// recorded in the lineage's row, and every later run of the lineage
/// installs that version, whatever this spec says, so a conversation
/// is never resumed against a plugin that changed under it. Each
/// [`settings`](Self::settings) entry and each [`secrets`](Self::secrets)
/// value read from the vault goes in the runtime's constructor
/// settings map and the entry process's environment both — the
/// core's `getSetting` never reads the environment, and some plugins
/// read nothing else. A package the registry cannot resolve, one that
/// is not a plugin once imported, or a secret the vault does not
/// hold refuses the run before the runtime starts.
///
/// The plugin runs in the agent's process with the caller's tools in
/// reach; the container is the sandbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Plugin {
    /// The npm spec: `@elizaos/plugin-discord`,
    /// `@elizaos/plugin-discord@2.0.4`, `@scope/anything`. A spec
    /// without a version resolves to the registry's latest on the
    /// lineage's first run, and to that resolved version after.
    pub package: String,
    /// The plugin's non-secret configuration, by the setting names
    /// the plugin reads — its own vocabulary, which the caller who
    /// chose it knows. JSON values; the harness renders each as the
    /// string Eliza's settings carry, and Eliza reads `"true"` and
    /// `"false"` back as booleans. Absent is nothing configured.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub settings: Map<String, Value>,
    /// The setting names that are secrets. Each IS the vault key the
    /// value lives under: the harness reads it from the vault and
    /// sets it as that setting. The value never appears in this
    /// document, in the character, in a row, or in the schema.
    /// Absent is no secrets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
}
