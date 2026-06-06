//! `agents tags add` — register a tag in PENDING state for an
//! `(agent_full_id, parent_agent_instance_hierarchy)` pair.
//!
//! On the next agent-completion spawn whose first chunk reports a
//! matching `(agent_full_id, parent)` pair, the tag is auto-promoted
//! to BOUND against the new agent_instance_hierarchy.
//!
//! `--parent-agent-instance-hierarchy` is optional; when omitted, the
//! CLI handler substitutes its own `Config.agent_instance_hierarchy`.
//! Mirrors the same fallback pattern as `agents message`.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.add.Request")]
pub struct Request {
    pub path_type: Path,
    pub name: String,
    pub agent_full_id: String,
    /// Optional parent scope. Falls back to
    /// `Config.agent_instance_hierarchy` at handler time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub parent_agent_instance_hierarchy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.add.Path")]
pub enum Path {
    #[serde(rename = "agents/tags/add")]
    AgentsTagsAdd,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "tags".to_string(),
            "add".to_string(),
            "--name".to_string(),
            self.name.clone(),
            "--agent-full-id".to_string(),
            self.agent_full_id.clone(),
        ];
        if let Some(parent) = &self.parent_agent_instance_hierarchy {
            argv.push("--parent-agent-instance-hierarchy".to_string());
            argv.push(parent.clone());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.add.Response")]
pub struct Response {
    pub name: String,
    pub agent_full_id: String,
    /// Resolved parent scope — filled in with the cli's own
    /// `Config.agent_instance_hierarchy` when the caller did not
    /// supply one.
    pub parent_agent_instance_hierarchy: String,
}

#[derive(clap::Args)]
pub struct Args {
    /// Tag name (unique). Re-using an existing tag displaces the
    /// previous binding silently.
    #[arg(long)]
    pub name: String,
    /// Agent full id this tag is waiting on.
    #[arg(long)]
    pub agent_full_id: String,
    /// Optional parent scope. When omitted, the CLI uses its own
    /// `Config.agent_instance_hierarchy`.
    #[arg(long = "parent-agent-instance-hierarchy")]
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
            path_type: Path::AgentsTagsAdd,
            name: args.name,
            agent_full_id: args.agent_full_id,
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
) -> Result<Response, E::Error> {
    request.jq = None;
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    jq: String,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    request.jq = Some(jq);
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
    }
}

pub mod request_schema;

pub mod response_schema;
