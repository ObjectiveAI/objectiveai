//! `tasks delete` — remove a task by id, resolved within a plugin
//! NAMESPACE. By default the namespace is the CALLER's own plugin
//! identity; `--no-plugin` targets the non-plugin (all-NULL)
//! namespace explicitly, and the `--plugin-owner`/`--plugin-name`/
//! `--plugin-version` trio targets a specific plugin's namespace.
//! This is plain identity resolution, NOT authentication — a caller
//! may name any namespace's id. Idempotent: deleting an absent id
//! reports `deleted: false`, never an error. A task deleted mid-run
//! finishes its in-flight run but records nothing and never fires
//! again.

use crate::cli::command::CommandRequest;

/// Which plugin namespace `tasks delete` resolves the id within.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "namespace", rename_all = "snake_case")]
#[schemars(rename = "cli.command.tasks.delete.DeleteNamespace")]
pub enum DeleteNamespace {
    /// Default: the caller's own plugin identity (the caller's trio,
    /// or the all-NULL namespace when the caller isn't a plugin).
    #[schemars(title = "Caller")]
    Caller,
    /// The non-plugin (all-NULL) namespace, explicitly.
    #[schemars(title = "NoPlugin")]
    NoPlugin,
    /// A specific plugin's namespace.
    #[schemars(title = "Plugin")]
    Plugin {
        owner: String,
        name: String,
        version: String,
    },
}

impl Default for DeleteNamespace {
    fn default() -> Self {
        Self::Caller
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.delete.Request")]
pub struct Request {
    pub path_type: Path,
    /// The task id.
    pub id: String,
    /// The namespace the id resolves within — defaults to the
    /// caller's own plugin identity.
    #[serde(default)]
    pub namespace: DeleteNamespace,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.delete.Path")]
pub enum Path {
    #[serde(rename = "tasks/delete")]
    TasksDelete,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.delete.Response")]
pub struct Response {
    /// Whether a task was actually removed (`false` = no such id).
    pub deleted: bool,
}

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("id_required").required(true).args(["id"])))]
pub struct Args {
    /// The task id.
    #[arg(long)]
    pub id: Option<String>,
    /// Resolve the id in the non-plugin (all-NULL) namespace instead
    /// of the caller's. Mutually exclusive with the plugin trio.
    #[arg(long, conflicts_with_all = ["plugin_owner", "plugin_name", "plugin_version"])]
    pub no_plugin: bool,
    /// Resolve the id in this plugin's namespace instead of the
    /// caller's. All three of owner/name/version are required
    /// together.
    #[arg(long, requires_all = ["plugin_name", "plugin_version"])]
    pub plugin_owner: Option<String>,
    #[arg(long, requires_all = ["plugin_owner", "plugin_version"])]
    pub plugin_name: Option<String>,
    #[arg(long, requires_all = ["plugin_owner", "plugin_name"])]
    pub plugin_version: Option<String>,
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
        let id = args.id.ok_or_else(|| {
            crate::cli::command::FromArgsError::path_parse("id", "--id is required".to_string())
        })?;
        // clap enforces the trio-together + no_plugin-exclusivity; the
        // match is exhaustive over the reachable combinations.
        let namespace = match (args.no_plugin, args.plugin_owner, args.plugin_name, args.plugin_version) {
            (true, _, _, _) => DeleteNamespace::NoPlugin,
            (false, Some(owner), Some(name), Some(version)) => {
                DeleteNamespace::Plugin { owner, name, version }
            }
            _ => DeleteNamespace::Caller,
        };
        Ok(Self {
            path_type: Path::TasksDelete,
            id,
            namespace,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        crate::cli::command::McpResponseItem::JSONL(serde_json::to_value(self).unwrap())
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

pub mod request_schema;

pub mod response_schema;

/// One `/listen` broadcast run of `tasks delete`: the actual
/// [`Request`], the producer's
/// [`AgentArguments`](crate::cli::command::AgentArguments), and the
/// unary response future. See [`crate::cli::broadcast_listener`].
#[cfg(feature = "cli-listener")]
pub struct ListenerExecution {
    pub request: Request,
    pub agent_arguments: crate::cli::command::AgentArguments,
    pub response: crate::cli::broadcast_listener::UnaryResponse<Response>,
}
