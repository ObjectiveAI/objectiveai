//! Wire format for the plugin command response channel.
//!
//! When a plugin emits a [`TypedPluginOutput::Command`] line on its
//! stdout, the host runs the command and pipes every emitted line back
//! into the plugin's stdin — wrapped in one of these envelopes per
//! line. The envelope is uniform whether the dispatching `Command`
//! carried an `id` or not, so the plugin's read loop parses the same
//! shape for every line.
//!
//! [`TypedPluginOutput::Command`]: super::TypedPluginOutput::Command

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::CommandResponseValue;

/// One envelope line the host writes to a plugin's stdin in response
/// to a previously-emitted [`TypedPluginOutput::Command`]. Plugins
/// read one of these per line; the same shape is used whether the
/// originating command had an `id` or not, so plugins can parse
/// uniformly. The terminal `command_complete` marker is just one of
/// the many possible notification payloads that ride through this
/// envelope — readers detect it by inspecting `value`.
///
/// Wire: `{"id":"<id>","value":<CommandResponseValue JSON>}` (with
/// `id` omitted when the dispatching command had no id).
///
/// [`TypedPluginOutput::Command`]: super::TypedPluginOutput::Command
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(rename = "cli.plugins.PluginCommandResponse")]
pub struct PluginCommandResponse {
    /// Echoed correlation id from the originating
    /// `TypedPluginOutput::Command.id`. Omitted on the wire when the
    /// dispatching command had no id; plugins that always mint ids
    /// can rely on this field being present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The wrapped line emitted by the spawned cli command — either a
    /// structured [`CommandResponseValue::Error`] or an opaque
    /// [`CommandResponseValue::Notification`] (which includes the
    /// terminal `CommandComplete` marker).
    pub value: CommandResponseValue,
}
