//! What this program writes to the entry.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use super::ReadKind;

/// One line to the entry's stdin.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// The runtime's construction, once, first: everything the entry
    /// hands the constructor. Camel-cased, as the entry reads it.
    #[serde(rename_all = "camelCase")]
    Configure {
        /// The lineage's agent id, pinned into the constructor.
        agent_id: String,
        /// The character, in Eliza's own shape.
        character: Value,
        /// The constructor settings map.
        settings: BTreeMap<String, String>,
        /// The caller's model-provider plugins, by package name, in
        /// priority order. Imported after the adapter and the diverge
        /// plugin, which the entry loads on its own; each
        /// must declare a model handler, and the entry refuses one
        /// that does not.
        model_provider_plugins: Vec<String>,
        /// The caller's other plugins, by package name. Imported
        /// last; each must declare no model handler, and the entry
        /// refuses one that does.
        plugins: Vec<String>,
        /// Whether the `GENERATE_MEDIA` action stays registered.
        generate_media: bool,
        /// Constructor option `advancedCapabilities`.
        advanced_capabilities: bool,
        /// Constructor option `enableRelationships`.
        enable_relationships: bool,
        /// Constructor option `enableDocuments`.
        enable_documents: bool,
        /// The proxy's MCP server, for the diverge plugin.
        mcp_url: String,
    },
    /// One turn: the input text.
    Turn { text: String },
    /// A rotating secret's read-back, answered by a `value` line.
    Read { kind: ReadKind, key: String },
    /// The end: stop the runtime, close the database, exit.
    Stop,
}
