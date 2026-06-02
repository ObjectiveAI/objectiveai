//! Wire format for the plugin output protocol.
//!
//! Plugins emit one [`Output`] JSON object per line on their stdout.
//! The host parses each line and dispatches per variant: `error` is
//! displayed, `mcp` announces the URL of an MCP server the plugin just
//! started (dispatched directly to the host's plugin-MCP-begin path),
//! `command` is a request for the host to perform some action and
//! (potentially) reply, and anything that doesn't match those three
//! lands in the untagged `Notification` catch-all and is forwarded to
//! whatever consumer the host has wired up.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Error, Mcp};

/// One line of plugin output. Untagged outer enum: deserialization
/// tries the three explicit [`TypedOutput`] variants first
/// (`type:"command" | "mcp" | "error"`), and falls through to
/// [`Output::Notification`] as a catch-all carrying the raw JSON value.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.plugins.Output")]
pub enum Output {
    #[schemars(title = "Typed")]
    Typed(TypedOutput),
    /// Final fallback — anything that didn't match a `Typed` variant
    /// lands here as an opaque JSON value. Hosts treat this as a
    /// notification payload to forward upstream.
    #[schemars(title = "Notification")]
    Notification(serde_json::Value),
}

/// The three explicitly-typed plugin output variants. Internally
/// tagged on `type`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
#[schemars(rename = "cli.plugins.TypedOutput")]
pub enum TypedOutput {
    #[schemars(title = "Command")]
    Command {
        /// Plugin-minted correlation id for this command. The host
        /// streams every emission from the command back into the
        /// plugin's stdin; plugins demultiplex concurrent in-flight
        /// commands by matching against the echoed id on each response
        /// line.
        id: String,
        command: String,
    },
    #[schemars(title = "Mcp")]
    Mcp(Mcp),
    #[schemars(title = "Error")]
    Error(Error),
}
