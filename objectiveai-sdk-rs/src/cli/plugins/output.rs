//! Wire format for the plugin output protocol.
//!
//! Plugins emit one [`Output`] JSON object per line on their stdout.
//! The host parses each line and dispatches per variant: `command`
//! is a request for the host to perform some action and (potentially)
//! reply, `mcp` announces the URL of an MCP server the plugin just
//! started, `error` is displayed, and anything that doesn't match
//! those three lands in the untagged [`Output::Notification`]
//! catch-all carrying the raw JSON value.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Command;
use crate::cli::Error;

/// One line of plugin output. Untagged outer enum — deserialization
/// tries each typed variant by its constant `type:"…"` discriminator
/// in source order and falls through to [`Output::Notification`] as
/// a catch-all carrying the raw JSON value.
///
/// The former `Mcp` announcement variant was removed with the
/// `plugins run` machinery — future plugins have no stdout protocol
/// at all; an MCP-URL announcement lands in the `Notification`
/// catch-all like any other untyped line.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.plugins.Output")]
pub enum Output {
    #[schemars(title = "Command")]
    Command(Command),
    #[schemars(title = "Error")]
    Error(Error),
    /// Final fallback — anything that didn't match a typed variant
    /// lands here as an opaque JSON value. Hosts treat this as a
    /// notification payload to forward upstream.
    #[schemars(title = "Notification")]
    Notification(serde_json::Value),
}
