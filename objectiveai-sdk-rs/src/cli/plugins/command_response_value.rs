//! Wire shape for the inner `value` of [`super::PluginCommandResponse`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Error;

/// One line wrapped inside a [`super::PluginCommandResponse`]: either a
/// structured [`Error`] or an opaque notification value the plugin
/// usually forwards upstream verbatim.
///
/// Untagged outer enum. `Error` is listed first so deserialization
/// tries it before the catch-all — `Error`'s `type:"error"` constant
/// short-circuits every non-error wire shape so notifications fall
/// through cleanly to [`CommandResponseValue::Notification`].
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.plugins.CommandResponseValue")]
pub enum CommandResponseValue {
    #[schemars(title = "Error")]
    Error(Error),
    /// Notification catch-all. Includes the terminal `CommandComplete`
    /// marker, which plugins detect by inspecting the JSON. Forwarded
    /// upstream verbatim by most plugin hosts.
    #[schemars(title = "Notification")]
    Notification(serde_json::Value),
}
