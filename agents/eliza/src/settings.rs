//! The agent, rendered: the settings map, the process environment
//! and the character the entry constructs the runtime with.
//!
//! Every setting and every secret goes in TWO places: the runtime's
//! constructor settings map — the core's `getSetting` never reads the
//! process environment, and `plugin-sql` reads `POSTGRES_URL` through
//! that path with no shim — and the entry process's environment, for
//! the plugins whose own helpers fall back to it. The map beats a
//! setting the `agents` row persisted, for every key it names, so the
//! value the proxy fed for a secret is always what the runtime sees.
//! Nothing goes on the character's `settings` or `secrets`, so
//! nothing lands in the row encrypted.
//!
//! The harness renders exactly one setting of its own, `POSTGRES_URL`,
//! for the database adapter the entry loads; every other setting is a
//! plugin's, given by the caller on that plugin's entry, and every
//! secret is the fed value under the name the caller named. Nothing
//! is rendered for a plugin the caller did not list, pre-installed or
//! not. The two values in the environment that are nobody's setting
//! — the vault passphrase and the settings salt — are the lineage's,
//! minted with its row.

use std::collections::BTreeMap;

use diverge_sdk::shared::containers::secrets::Values;
use serde_json::Value;

use crate::agent::{Agent, Character};

/// Where Eliza keeps its own state on disk — its vault under
/// `.vault-pglite/` above all. Fixed: a caller who wants it to outlive
/// the container mounts a volume here.
pub const STATE_DIR: &str = "/var/lib/eliza";

/// The passphrase Eliza's own vault derives its master key from:
/// the lineage's, minted with its row, and the process environment's
/// under this name.
pub const PASSPHRASE: &str = "ELIZA_VAULT_PASSPHRASE";

/// The salt Eliza encrypts settings with, the lineage's under this
/// name. Stable for the lineage: `initialize()` merges the `agents`
/// row's settings back into the character, and a decrypt under a
/// different salt throws.
pub const SALT: &str = "SECRET_SALT";

/// What the entry constructs the runtime with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The constructor settings map.
    pub settings: BTreeMap<String, String>,
    /// The entry process's environment: the map, plus the process-only
    /// keys (the state directory, the vault passphrase and its
    /// keychain switch, the salt, trajectory logging off).
    pub env: BTreeMap<String, String>,
    /// The character, in Eliza's own casing, with the memory flags.
    pub character: Value,
    /// Whether the `GENERATE_MEDIA` action stays registered.
    pub generate_media: bool,
    /// Constructor option `advancedCapabilities`.
    pub advanced_capabilities: bool,
    /// Constructor option `enableRelationships`.
    pub enable_relationships: bool,
    /// Constructor option `enableDocuments`.
    pub enable_documents: bool,
}

/// Render the agent with the secrets the proxy fed (fixed and
/// rotating alike, by setting name, each as the text a setting is),
/// the database URL, and the lineage's passphrase and salt.
pub fn render(
    agent: &Agent,
    secrets: &Values,
    postgres_url: String,
    passphrase: &str,
    salt: &str,
) -> Rendered {
    let mut settings: BTreeMap<String, String> = BTreeMap::new();

    settings.insert("POSTGRES_URL".to_string(), postgres_url);

    // Every plugin's own settings, the model providers' first: a
    // later entry's setting of the same name wins, which is the
    // caller's order to know.
    for plugin in agent.model_provider_plugins.iter().chain(&agent.plugins) {
        for (key, value) in &plugin.settings {
            if let Some(text) = text(value) {
                settings.insert(key.clone(), text);
            }
        }
    }

    // The fed values last, so a plugin setting cannot shadow a
    // secret of the same name.
    for (key, value) in secrets {
        if let Some(text) = text(value) {
            settings.insert(key.clone(), text);
        }
    }

    let mut env = settings.clone();
    env.insert("ELIZA_STATE_DIR".to_string(), STATE_DIR.to_string());
    env.insert(PASSPHRASE.to_string(), passphrase.to_string());
    env.insert("ELIZA_VAULT_DISABLE_KEYCHAIN".to_string(), "1".to_string());
    env.insert(SALT.to_string(), salt.to_string());
    env.insert(
        "ELIZA_DISABLE_TRAJECTORY_LOGGING".to_string(),
        "1".to_string(),
    );

    let memory = &agent.memory;
    Rendered {
        settings,
        env,
        character: character(&agent.character, memory.advanced_memory, memory.advanced_planning),
        generate_media: agent.generate_media == Some(true),
        advanced_capabilities: memory.advanced_capabilities == Some(true),
        enable_relationships: memory.relationships == Some(true),
        enable_documents: memory.documents == Some(true),
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
/// to set. A fed secret takes the same shape, so a rotated value read
/// back is compared against its fed value rendered this way.
pub fn text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Bool(bool) => Some(bool.to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Array(_) | Value::Object(_) => Some(value.to_string()),
    }
}
