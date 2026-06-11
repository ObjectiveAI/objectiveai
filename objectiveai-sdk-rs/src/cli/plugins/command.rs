//! Wire shape for a plugin-emitted command-execution request.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Plugin requests the host execute a command. The host streams
/// every emission back into the plugin's stdin; plugins demultiplex
/// concurrent in-flight commands by matching against the echoed
/// `id` on each response line.
///
/// The constant `type:"command"` discriminator disambiguates this
/// variant from the rest of the untagged [`super::Output`] catch-all,
/// mirroring the `type:"error"` discriminator on
/// [`crate::cli::Error`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "cli.plugins.Command")]
pub struct Command {
    pub r#type: CommandType,
    /// Plugin-minted correlation id. Echoed by the host on every
    /// response line so the plugin can demux concurrent calls.
    pub id: String,
    /// The command to run, as an already-tokenized argv vector — one
    /// element per argument. Carried structured (NOT a space-joined
    /// string) so an argument whose value contains whitespace (e.g.
    /// `["agents", "message", "leaf", "--simple", "a b c"]`) keeps its
    /// boundary intact; the host dispatches it verbatim without
    /// re-tokenizing.
    pub command: Vec<String>,
}

/// Single-variant discriminator for [`Command`]'s `type` field.
/// Always `"command"` on the wire.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.plugins.CommandType")]
pub enum CommandType {
    Command,
}
