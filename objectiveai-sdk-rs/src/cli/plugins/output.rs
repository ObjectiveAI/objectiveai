//! Wire format for the plugin output protocol.
//!
//! Plugins emit one [`PluginOutput`] JSON object per line on their
//! stdout. The host parses each line and dispatches per variant:
//! `error` is displayed, `notification` is forwarded to whatever
//! consumer the host has wired up, `command` is a request for the
//! host to perform some action and (potentially) reply.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use crate::cli::output::{Error, Level};

/// One line of plugin output.
///
/// Identical in shape to [`crate::cli::output::Output`] except:
///
/// - [`PluginOutput::Notification`] is a plain `serde_json::Value`
///   (no generic `T`, no nesting wrapper). The plugin is responsible
///   for not including `"type"` as a top-level key in the value,
///   which would collide with the discriminator.
/// - No `Begin`/`End` markers — plugins don't bracket their stream.
/// - Adds [`PluginOutput::Command`] — a request the host should act
///   on, identified by a `command` string.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.plugins.PluginOutput")]
pub enum PluginOutput {
    #[schemars(title = "Error")]
    Error(Error),
    #[schemars(title = "Notification")]
    Notification(serde_json::Value),
    #[schemars(title = "Command")]
    Command { command: String },
}
