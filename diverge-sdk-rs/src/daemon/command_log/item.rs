//! What one entry of a command log holds.

use serde::{Deserialize, Serialize};

use super::Command;

/// One item of a command log, told apart by `type`: a command, the one
/// kind there is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    /// A request put to the daemon: `command`.
    Command(Command),
}
