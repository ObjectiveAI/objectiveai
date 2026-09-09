//! The agent, rendered: the settings map, the process environment,
//! the plugin list and the character the entry constructs the runtime
//! with.
//!
//! Every setting and every secret goes in TWO places: the runtime's
//! constructor settings map — the core's `getSetting` never reads the
//! process environment, and `plugin-sql` reads `POSTGRES_URL` through
//! that path with no shim — and the entry process's environment, for
//! the plugins whose own helpers fall back to it. The map beats a
//! setting the `agents` row persisted, for every key it names, so the
//! vault's copy of a secret is always what the runtime sees. Nothing
//! goes on the character's `settings` or `secrets`, so nothing lands
//! in the row encrypted.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::agent::{Agent, Character};
use crate::vault;

/// Where Eliza keeps its own state on disk — its vault under
/// `.vault-pglite/` above all. Fixed: a caller who wants it to outlive
/// the container mounts a volume here.
pub const STATE_DIR: &str = "/var/lib/eliza";

/// Where the documents feature reads the caller's files from, when
/// the `documents` toolset is on: a mount, at this path.
pub const DOCUMENTS_PATH: &str = "/documents";

/// The coding tools' workspace root and the shell's allowed directory:
/// the whole container, which is the sandbox.
pub const WORKSPACE: &str = "/";

/// The image's plugins, in registration order, by switch.
const SQL: &str = "@elizaos/plugin-sql";
const OPENAI: &str = "@elizaos/plugin-openai";
const EMBEDDINGS: &str = "@elizaos/plugin-embeddings";
const CODING_TOOLS: &str = "@elizaos/plugin-coding-tools";
const BROWSER: &str = "@elizaos/plugin-browser";
const DOCUMENTS: &str = "@elizaos/plugin-documents";
const WEB_SEARCH: &str = "@elizaos/plugin-web-search";

/// What the entry constructs the runtime with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The constructor settings map.
    pub settings: BTreeMap<String, String>,
    /// The entry process's environment: the map, plus the process-only
    /// keys (the state directory, the vault passphrase and its
    /// keychain switch, the salt, trajectory logging off).
    pub env: BTreeMap<String, String>,
    /// The image's plugins to load, in order.
    pub plugins: Vec<String>,
    /// The character, in Eliza's own casing, with the memory flags.
    pub character: Value,
    /// Whether `plugin-openai` registers its media tiers.
    pub openai_media: bool,
    /// Constructor option `advancedCapabilities`.
    pub advanced_capabilities: bool,
    /// Constructor option `enableRelationships`.
    pub enable_relationships: bool,
    /// Constructor option `enableDocuments`.
    pub enable_documents: bool,
}

/// Render the agent with the secrets the vault gave (static and
/// rotating alike, by setting name), the database URL, and the two
/// minted process secrets.
pub fn render(
    agent: &Agent,
    secrets: &BTreeMap<String, String>,
    postgres_url: String,
    passphrase: &str,
    salt: &str,
) -> Rendered {
    let mut settings: BTreeMap<String, String> = BTreeMap::new();
    let mut plugins = vec![SQL.to_string(), OPENAI.to_string()];

    settings.insert("POSTGRES_URL".to_string(), postgres_url);

    settings.insert("OPENAI_BASE_URL".to_string(), agent.provider.base_url.clone());
    for tier in [
        "OPENAI_NANO_MODEL",
        "OPENAI_SMALL_MODEL",
        "OPENAI_MEDIUM_MODEL",
        "OPENAI_LARGE_MODEL",
        "OPENAI_MEGA_MODEL",
    ] {
        settings.insert(tier.to_string(), agent.provider.model.clone());
    }

    if let Some(embedding) = &agent.embedding {
        plugins.push(EMBEDDINGS.to_string());
        settings.insert("EMBEDDING_BASE_URL".to_string(), embedding.base_url.clone());
        settings.insert("EMBEDDING_MODEL".to_string(), embedding.model.clone());
        settings.insert(
            "EMBEDDING_DIMENSIONS".to_string(),
            embedding.dimensions.to_string(),
        );
    }

    let toolsets = &agent.toolsets;
    if toolsets.coding_tools == Some(true) {
        plugins.push(CODING_TOOLS.to_string());
        settings.insert(
            "CODING_TOOLS_WORKSPACE_ROOTS".to_string(),
            WORKSPACE.to_string(),
        );
        settings.insert("SHELL_ALLOWED_DIRECTORY".to_string(), WORKSPACE.to_string());
    }
    if toolsets.browser == Some(true) {
        plugins.push(BROWSER.to_string());
    }
    let enable_documents = toolsets.documents == Some(true);
    if enable_documents {
        plugins.push(DOCUMENTS.to_string());
        settings.insert("DOCUMENTS_PATH".to_string(), DOCUMENTS_PATH.to_string());
        settings.insert("LOAD_DOCS_ON_STARTUP".to_string(), "true".to_string());
    }
    if toolsets.web_search == Some(true) {
        plugins.push(WEB_SEARCH.to_string());
    }

    for plugin in &agent.plugins {
        for (key, value) in &plugin.settings {
            if let Some(text) = text(value) {
                settings.insert(key.clone(), text);
            }
        }
    }

    // The vault's values last, so a plugin setting cannot shadow a
    // secret of the same name.
    for (key, value) in secrets {
        settings.insert(key.clone(), value.clone());
    }

    let mut env = settings.clone();
    env.insert("ELIZA_STATE_DIR".to_string(), STATE_DIR.to_string());
    env.insert(vault::PASSPHRASE.to_string(), passphrase.to_string());
    env.insert("ELIZA_VAULT_DISABLE_KEYCHAIN".to_string(), "1".to_string());
    env.insert(vault::SALT.to_string(), salt.to_string());
    env.insert(
        "ELIZA_DISABLE_TRAJECTORY_LOGGING".to_string(),
        "1".to_string(),
    );

    let memory = &agent.memory;
    Rendered {
        settings,
        env,
        plugins,
        character: character(&agent.character, memory.advanced_memory, memory.advanced_planning),
        openai_media: toolsets.generate_media == Some(true),
        advanced_capabilities: memory.advanced_capabilities == Some(true),
        enable_relationships: memory.relationships == Some(true),
        enable_documents,
    }
}

/// The character in Eliza's own shape: camel-cased, the examples
/// grouped as Eliza groups them, the two memory flags on it.
fn character(character: &Character, advanced_memory: Option<bool>, advanced_planning: Option<bool>) -> Value {
    let mut value = serde_json::json!({
        "name": character.name,
        "bio": character.bio,
        "topics": character.topics,
        "adjectives": character.adjectives,
        "style": {
            "all": character.style.all,
            "chat": character.style.chat,
            "post": character.style.post,
        },
        "messageExamples": character
            .message_examples
            .iter()
            .map(|group| {
                serde_json::json!({
                    "examples": group
                        .iter()
                        .map(|example| {
                            serde_json::json!({
                                "name": example.name,
                                "content": { "text": example.text },
                            })
                        })
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
    });
    let object = value.as_object_mut().expect("built as an object");
    if let Some(system) = &character.system {
        object.insert("system".to_string(), Value::String(system.clone()));
    }
    if advanced_memory == Some(true) {
        object.insert("advancedMemory".to_string(), Value::Bool(true));
    }
    if advanced_planning == Some(true) {
        object.insert("advancedPlanning".to_string(), Value::Bool(true));
    }
    value
}

/// A setting value as the string Eliza carries: text as-is, booleans
/// and numbers spelled out (Eliza reads `"true"` and `"false"` back as
/// booleans), objects and arrays as their JSON, and null as nothing
/// to set.
fn text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Bool(bool) => Some(bool.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}
