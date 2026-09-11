//! Where a plugin leaves a rotated credential.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where the harness reads a rotated value back from, after every
/// turn and at the run's end — the three channels an elizaOS plugin
/// has for a credential it rotates. Untagged.
///
/// ```json
/// true                                   // the setting of the same name
/// {"setting": "GOOGLE_TOKENS"}           // another setting
/// {"file": ["root", ".qwen", "oauth_creds.json"]}
/// {"vault": "agent.{agent_id}.wallet.evm"}   // Eliza's own vault
/// ```
///
/// A setting is read through the runtime the harness constructs —
/// `getSetting`, which a plugin wrote with `setSetting` — and a
/// non-string value goes to the vault as its JSON; a file goes as
/// its bytes; an entry of Eliza's own vault is read through the
/// `@elizaos/vault` library with the master key the harness holds,
/// and goes as its decrypted value. Nothing else is a channel:
/// Eliza's cache and its tables are the database, and a credential
/// does not live there (see [`Secret`](super::Secret)). A read-back that finds nothing
/// new is not an error — the plugin may not have rotated — but a
/// read-back that finds nothing at all is a warning at the run's
/// end, naming the key, so the caller learns their copy may be stale
/// before the next login fails.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Rotates {
    /// `true`: the plugin rotates the setting in place, so the
    /// rotated value is the setting of the same name. `false` is a
    /// static secret said the long way.
    InPlace(bool),
    /// The plugin leaves the rotated value under another setting.
    Setting {
        /// The setting's name.
        setting: String,
    },
    /// The plugin leaves the rotated value in a file.
    File {
        /// The file's path, as components from the container's root
        /// — the shape every path in this system takes.
        file: Vec<String>,
    },
    /// The plugin leaves the rotated value in Eliza's own vault.
    Vault {
        /// The entry's key, as the plugin names it. `{agent_id}` in
        /// it is replaced with the lineage's agent id, which the
        /// harness minted and the caller cannot know — the wallet
        /// plugin's `agent.<agentId>.wallet.<chain>` is the shape.
        vault: String,
    },
}
