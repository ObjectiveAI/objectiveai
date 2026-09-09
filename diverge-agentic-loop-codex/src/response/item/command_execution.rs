//! The `command_execution` item: a command the agent ran.

use serde::Deserialize;

/// A command executed by the agent: started when spawned (output
/// empty, no exit code, in progress), completed when the process
/// exits.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CommandExecution {
    /// The command line.
    pub command: String,
    /// Stdout and stderr, interleaved as they came.
    pub aggregated_output: String,
    /// The exit code, once there is one.
    pub exit_code: Option<i32>,
    /// Where the command stands.
    pub status: CommandExecutionStatus,
}

/// The status of a command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutionStatus {
    /// Running.
    #[default]
    InProgress,
    /// Exited zero.
    Completed,
    /// Exited non-zero, or could not run.
    Failed,
    /// Refused by the approval policy — never here, where the policy
    /// is `never`.
    Declined,
}
