//! `agents message-queue read pending` ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â async handler stub.
//!
//! Streams every queued prompt whose target is a direct child of
//! the given parent. Direct rows match when their
//! `agent_instance_hierarchy` is one segment under `parent` ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â same
//! filter `agents list active` uses. Tag rows resolve their parent
//! via the joined `tags` table:
//!
//! * BOUND tags ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ parent is `parent_of(bound_agent_instance_hierarchy)`.
//! * PENDING tags ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â‚¬Å¾Ã‚Â¢ parent is the stored
//!   `parent_agent_instance_hierarchy` from the tags row.
//! * ABSENT tags (the tag was used at enqueue but never registered)
//!   have no parent and are excluded.
//!
//! Each tag-row response item carries the joined 3-state status
//! (`Bound { hierarchy } | Pending { ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¦ }`) for inspection.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.read.pending.Request")]
pub struct Request {
    pub path_type: Path,
    pub parent_agent_instance_hierarchy: Option<String>,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.message_queue.read.pending.Path")]
pub enum Path {
    #[serde(rename = "agents/message-queue/read/pending")]
    AgentsMessageQueueReadPending,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "message-queue".to_string(),
            "read".to_string(),
            "pending".to_string(),
        ];
        if let Some(p) = &self.parent_agent_instance_hierarchy {
            argv.push(p.clone());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

/// One queued prompt. Direct rows carry only the bare
/// `agent_instance` (= leaf segment of the hierarchy); Tag rows
/// carry the literal tag name and flatten the joined 2-state
/// status onto the same JSON object ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â yielding e.g.
/// `{"by":"tag","id":42,"agent_tag":"foo","state":"bound","agent_instance_hierarchy":"ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¦",
/// "content":17}` (single-part) or `ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¦"content":[17,18,19]"` (multi-
/// part) rather than nesting the state under its own object.
///
/// Both variants carry the resolved content body as a
/// [`super::super::super::instances::read::all::ResponseContent`] ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â `One(i64)` for a
/// single-part `RichContent::Text` (or single-element
/// `RichContent::Parts`), `Many(Vec<i64>)` for multi-part payloads.
/// Each id is a `prompt_contents.id` resolvable via
/// `agents message-queue read id`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "by", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.message_queue.read.pending.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "AgentInstance")]
    AgentInstance {
        id: i64,
        agent_instance: String,
        /// Idempotency token, if the row was enqueued with `--key`.
        /// Skipped from the wire shape when `None`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        content: super::super::super::instances::read::all::ResponseContent,
    },
    #[schemars(title = "Tag")]
    Tag {
        id: i64,
        agent_tag: String,
        #[serde(flatten)]
        state: LookupState,
        /// Idempotency token, if the row was enqueued with `--key`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(extend("omitempty" = true))]
        key: Option<String>,
        content: super::super::super::instances::read::all::ResponseContent,
    },
}

// Reuse the same `LookupState` enum that `agents tags lookup`
// already exposes ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â same wire shape, same Rust type, no fork.
pub use super::super::super::tags::lookup::LookupState;

#[derive(clap::Args)]
pub struct Args {
    /// Filter both Direct and Tag rows to direct children of this
    /// parent. Tags resolve their parent via the joined `tags`
    /// table (BOUND tags by `parent_of(bound_hierarchy)`, PENDING
    /// tags by their stored `parent_agent_instance_hierarchy`).
    /// Omit for the cli's own position.
    pub parent_agent_instance_hierarchy: Option<String>,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        Ok(Self {
            path_type: Path::AgentsMessageQueueReadPending,
            parent_agent_instance_hierarchy: args.parent_agent_instance_hierarchy,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.jq = None;
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.jq = Some(jq);
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
