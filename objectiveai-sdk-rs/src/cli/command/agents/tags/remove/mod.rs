//! `agents tags remove` — delete a tag registration by name,
//! whatever its shape (BOUND or GROUPED).
//!
//! Removal also detaches the tag's laboratory attachments in the same
//! transaction (attachments target existing tags — leaving orphans
//! would silently resurrect them if the name were ever reused), and
//! garbage-collects a GROUPED tag's `tag_groups` row when this was
//! its last member. A tag whose name a live process owns (the same
//! in-process lock `apply` honors) is rejected; a missing tag is an
//! error, not a no-op — matching the apply/detach convention.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.remove.Path")]
pub enum Path {
    #[serde(rename = "agents/tags/remove")]
    AgentsTagsRemove,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.remove.Request")]
pub struct Request {
    pub path_type: Path,
    /// Tag name to remove (the global registry key).
    pub tag: String,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.remove.Response")]
pub struct Response {
    /// The removed tag's name, echoed.
    pub name: String,
    /// What the tag was at removal time.
    #[serde(flatten)]
    pub removed: Removed,
    /// Laboratory attachments targeting this tag that were detached
    /// with it.
    pub detached_laboratories: u64,
}

/// The removed tag's shape — mirrors `lookup`'s two live states (a
/// missing tag is an error, not a state).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tags.remove.Removed")]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Removed {
    /// The tag was BOUND to this hierarchy.
    #[schemars(title = "Bound")]
    Bound { agent_instance_hierarchy: String },
    /// The tag was GROUPED. `tag_group_deleted` reports whether this
    /// removal emptied — and so garbage-collected — the group.
    #[schemars(title = "Grouped")]
    Grouped { tag_group_deleted: bool },
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(clap::Args)]
#[command(group(
    clap::ArgGroup::new("tag_required")
        .required(true)
        .args(["tag"])
))]
pub struct Args {
    /// Tag name to remove.
    #[arg(long)]
    pub tag: Option<String>,
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
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
        let tag = args
            .tag
            .expect("clap group `tag_required` ensures --tag is present");
        Ok(Request {
            path_type: Path::AgentsTagsRemove,
            tag,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<Response, E::Error> {
    request.base.clear_transform();
    executor.execute_one(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<serde_json::Value, E::Error> {
    request.base.set_transform(transform);
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

/// One `/listen` broadcast run of `agents tags remove`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
