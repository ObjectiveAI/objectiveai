//! `agents logs read subscribe` — the live cousin of
//! `agents logs read pending`. Same `Vec<Target>` input + same
//! parts-grouped block output, but with a first-ping-or-go-inactive
//! wait loop. Returns either a real block (the EXACT same JSON
//! shape `read all` / `read pending` emit) OR the literal
//! string `"agents_inactive"` when no target has a live agent
//! to wait on.
//!
//! Optional kind filter: any subset of `--request` /
//! `--assistant` / `--tool` flags. `--request` covers BOTH
//! request blob rows AND `message_queue_*` client notification
//! rows (incoming work is one bucket regardless of source). No
//! flags set = no filter (all kinds).

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.read.subscribe.Request")]
pub struct Request {
    pub path_type: Path,
    pub targets: Vec<Target>,
    /// Filter to rows whose `MessageTable` falls in the selected
    /// bucket(s). `None` on the wire = no filter (all kinds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub kinds: Option<KindFilter>,
    /// Skip rows with `logs.messages."index" <= after_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub limit: Option<i64>,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
}

/// 3-bool bitset for the type-filter flags. All independent and
/// optional. If all 3 are false, the request's `kinds` field is
/// serialized as `None` (no filter — wait for any kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.read.subscribe.KindFilter")]
pub struct KindFilter {
    /// `--request`: covers all 3 request blob kinds
    /// (`agent_completion_request` / `vector_completion_request`
    /// / `function_execution_request`) AND the 5
    /// `message_queue_*` client notification kinds. Treats
    /// "incoming work" as one bucket regardless of whether it's
    /// a new completion request or a queued message landing in
    /// the agent's inbox.
    pub request: bool,
    /// `--assistant`: all 8 `assistant_response_*` kinds
    /// (refusal / reasoning / tool_calls + 5 content kinds).
    pub assistant: bool,
    /// `--tool`: all 6 `tool_response*` kinds (1 container +
    /// 5 content kinds).
    pub tool: bool,
}

impl KindFilter {
    /// Returns `None` when all 3 booleans are false (no filter
    /// — `Request.kinds` stays `None` on the wire).
    pub fn from_flags(request: bool, assistant: bool, tool: bool) -> Option<Self> {
        if request || assistant || tool {
            Some(Self { request, assistant, tool })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.read.subscribe.Path")]
pub enum Path {
    #[serde(rename = "agents/logs/read/subscribe")]
    AgentsLogsReadSubscribe,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "logs".to_string(),
            "read".to_string(),
            "subscribe".to_string(),
        ];
        for target in &self.targets {
            argv.push("--target".to_string());
            argv.push(target.into_arg_string());
        }
        if let Some(kinds) = &self.kinds {
            if kinds.request {
                argv.push("--request".to_string());
            }
            if kinds.assistant {
                argv.push("--assistant".to_string());
            }
            if kinds.tool {
                argv.push("--tool".to_string());
            }
        }
        if let Some(after_id) = self.after_id {
            argv.push("--after-id".to_string());
            argv.push(after_id.to_string());
        }
        if let Some(limit) = self.limit {
            argv.push("--limit".to_string());
            argv.push(limit.to_string());
        }
        self.base.push_flags(&mut argv);
        argv
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

// Re-export `Target` from `logs::read::all` — single source of
// truth for the docker-style `--target` parser. Same shape on
// the wire as `agents logs read all` / `read pending`.
pub use super::all::Target;

/// Subscribe's wire shape. Either a real parts-grouped block
/// (the EXACT same enum `read all` / `read pending` emit) OR
/// the literal string `"agents_inactive"`. `#[serde(untagged)]`
/// so the `Item` arm passes through transparently — JSONL
/// consumers see either a block JSON object or a bare string.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.logs.read.subscribe.ResponseItem")]
pub enum ResponseItem {
    #[schemars(title = "Item")]
    Item(super::all::ResponseItem),
    #[schemars(title = "AgentsInactive")]
    AgentsInactive(AgentsInactiveTag),
}

/// Single-variant enum whose lone variant serializes as the
/// literal string `"agents_inactive"`. Construct as
/// `AgentsInactiveTag::AgentsInactive`; the surrounding
/// `ResponseItem::AgentsInactive(_)` carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.logs.read.subscribe.AgentsInactiveTag")]
pub enum AgentsInactiveTag {
    #[serde(rename = "agents_inactive")]
    AgentsInactive,
}

#[derive(clap::Args)]
pub struct Args {
    /// One or more `--target instance=L[,parent=P]` entries.
    /// `parent` defaults to the cli's own
    /// `Config.agent_instance_hierarchy` when omitted on an
    /// individual target. Also accepts `--target tag=T` and
    /// `--target me` (the caller's own AIH).
    #[arg(long = "target", required = true)]
    pub targets: Vec<String>,
    /// Wait for new request blob rows OR new `message_queue_*`
    /// client notification rows.
    #[arg(long)]
    pub request: bool,
    /// Wait for new `assistant_response_*` rows.
    #[arg(long)]
    pub assistant: bool,
    /// Wait for new `tool_response*` rows.
    #[arg(long)]
    pub tool: bool,
    /// Skip rows with `logs.messages."index" <= after_id`.
    #[arg(long)]
    pub after_id: Option<i64>,
    /// Cap on rows scanned per target.
    #[arg(long)]
    pub limit: Option<i64>,
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
        let targets = args
            .targets
            .iter()
            .map(|s| {
                s.parse::<Target>().map_err(|msg| {
                    crate::cli::command::FromArgsError::path_parse("target", msg)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let kinds = KindFilter::from_flags(args.request, args.assistant, args.tool);
        Ok(Self {
            path_type: Path::AgentsLogsReadSubscribe,
            targets,
            kinds,
            after_id: args.after_id,
            limit: args.limit,
            base: args.base.into(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<ResponseItem>, E::Error> {
    request.base.clear_transform();
    executor.execute(request, agent_arguments).await
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    mut request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<E::Stream<serde_json::Value>, E::Error> {
    request.base.set_transform(transform);
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
