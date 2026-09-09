//! One secret a plugin reads, and whether the run rotates it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Rotates;

/// A secret the plugin reads: a setting name that IS the vault key
/// the value lives under.
///
/// Untagged: a bare string is a static secret, read from the vault
/// once per run and set as that setting — the common case, an API
/// key. An object names a secret the run ROTATES — an OAuth login
/// whose refresh token is single-use — and where the plugin leaves
/// the rotated value, so the harness can put it back: the key is
/// locked for the run, read, set as the setting, read back after
/// every turn and at the end from the place [`rotates`](Self::Rotating)
/// names, set into the vault, and unlocked when the run ends. The
/// value never appears in this document, in the character, or in
/// the schema.
///
/// # Credentials never live in the database
///
/// The rotated value's home is the vault, never Eliza's database:
/// the caller's database will be snapshotted and rewound, and a
/// credential rewound is a login burned. Eliza is built on the same
/// expectation — a plugin keeps its credentials in settings, not in
/// tables — and this container assumes it: a plugin that persists a
/// rotating credential in a table of its own cannot be supported.
/// The three places a plugin may leave a rotated value are its
/// setting, a file, and Eliza's own vault — the encrypted store the
/// `@elizaos/vault` library keeps on disk, on by default with a
/// passphrase the harness holds — and the harness re-applies the
/// vault's copy at the top of Eliza's precedence at every run's
/// start, so a rewound row never wins over the vault.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Secret {
    /// A static secret: the vault key, read once per run, set as
    /// the setting of the same name, never written back.
    Static(String),
    /// A secret the run rotates.
    Rotating {
        /// The setting name, and the vault key.
        key: String,
        /// Where the plugin leaves the rotated value.
        rotates: Rotates,
    },
}
