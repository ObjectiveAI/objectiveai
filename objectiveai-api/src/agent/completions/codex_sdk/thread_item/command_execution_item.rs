use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutionItemType {
    CommandExecution,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandExecutionItem {
    pub id: String,
    pub r#type: CommandExecutionItemType,
    pub command: String,
    pub aggregated_output: String,
    /// Set when the command exits; omitted while still running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
    pub status: super::CommandExecutionStatus,
}
