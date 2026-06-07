//! `agents read id` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.id.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.instances.read.id.Path")]
pub enum Path {
    #[serde(rename = "agents/instances/read/id")]
    AgentsInstancesReadId,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
            "instances".to_string(),
            "read".to_string(),
            "id".to_string(),
            self.id.to_string(),
        ];
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

// Adjacently tagged on purpose — this union carries several
// all-`Option` payload shapes (`Logprobs`, `File`) that deserialize
// from ANY JSON object, so an untagged walk misclassifies whichever
// payload comes after them (a tool-call delta re-materialized as an
// empty `Logprobs`). The `type` value is the variant's schemars
// title in snake_case; the payload rides under `value`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[schemars(rename = "cli.command.agents.instances.read.id.Response")]
pub enum Response {
    // Typed log envelopes — each variant name is the PascalCase form
    // of its full on-disk path under `logs/`. Payloads alias the
    // matching `… ::get::Response` where a command leaf exists;
    // role-subdir shapes with no leaf carry the SDK log type the
    // writer serialized, verbatim.
    #[schemars(title = "AgentsCompletionsResponse")]
    AgentsCompletionsResponse(crate::cli::command::logs::agents::completions::response::get::Response),
    #[schemars(title = "AgentsCompletionsRequest")]
    AgentsCompletionsRequest(crate::cli::command::logs::agents::completions::request::get::Response),
    #[schemars(title = "AgentsCompletionsResponseMessagesAssistant")]
    AgentsCompletionsResponseMessagesAssistant(crate::cli::command::logs::agents::completions::response::messages::assistant::get::Response),
    #[schemars(title = "AgentsCompletionsResponseMessagesTool")]
    AgentsCompletionsResponseMessagesTool(crate::cli::command::logs::agents::completions::response::messages::tool::get::Response),
    #[schemars(title = "AgentsCompletionsRequestMessages")]
    AgentsCompletionsRequestMessages(crate::agent::completions::message::MessageLog),
    #[schemars(title = "AgentsCompletionsResponseMessagesAssistantLogprobs")]
    AgentsCompletionsResponseMessagesAssistantLogprobs(crate::cli::command::logs::agents::completions::response::messages::assistant::logprobs::get::Response),
    #[schemars(title = "AgentsCompletionsResponseMessagesAssistantToolCalls")]
    AgentsCompletionsResponseMessagesAssistantToolCalls(crate::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::Response),
    // Request-side tool calls are written as full `AssistantToolCall`s
    // (no `index`), unlike the response side's streaming deltas.
    #[schemars(title = "AgentsCompletionsRequestMessagesAssistantToolCalls")]
    AgentsCompletionsRequestMessagesAssistantToolCalls(crate::agent::completions::message::AssistantToolCall),

    #[schemars(title = "VectorCompletionsResponse")]
    VectorCompletionsResponse(crate::cli::command::logs::vector::completions::response::get::Response),
    #[schemars(title = "VectorCompletionsRequest")]
    VectorCompletionsRequest(crate::cli::command::logs::vector::completions::request::get::Response),

    #[schemars(title = "FunctionsExecutionsResponse")]
    FunctionsExecutionsResponse(crate::cli::command::logs::functions::executions::response::get::Response),
    #[schemars(title = "FunctionsExecutionsRequest")]
    FunctionsExecutionsRequest(crate::cli::command::logs::functions::executions::request::get::Response),

    // Collapsed text/media — one variant per content kind, regardless
    // of where the file lives.
    #[schemars(title = "Text")]
    Text(String),
    #[schemars(title = "Image")]
    Image(crate::agent::completions::message::ImageUrl),
    #[schemars(title = "Audio")]
    Audio(crate::agent::completions::message::InputAudio),
    #[schemars(title = "Video")]
    Video(crate::agent::completions::message::VideoUrl),
    #[schemars(title = "File")]
    File(crate::agent::completions::message::File),
}

#[derive(clap::Args)]
pub struct Args {
    /// Log row id.
    pub id: i64,
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
        Ok(Self { path_type: Path::AgentsInstancesReadId,
            id: args.id,
            jq: args.jq,
        })
    }
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        // Every variant's payload type already implements
        // `CommandResponse`, so each arm delegates straight through:
        // media variants pick up `Media(ContentBlock)`, the typed
        // log envelopes pick up `JSONL(serde_value)`, and `Text(String)`
        // picks up the `Value::String` shortcut from the `String` impl.
        match self {
            Response::AgentsCompletionsResponse(v) => v.into_mcp(),
            Response::AgentsCompletionsRequest(v) => v.into_mcp(),
            Response::AgentsCompletionsResponseMessagesAssistant(v) => v.into_mcp(),
            Response::AgentsCompletionsResponseMessagesTool(v) => v.into_mcp(),
            Response::AgentsCompletionsRequestMessages(v) => v.into_mcp(),
            Response::AgentsCompletionsResponseMessagesAssistantLogprobs(v) => v.into_mcp(),
            Response::AgentsCompletionsResponseMessagesAssistantToolCalls(v) => v.into_mcp(),
            Response::AgentsCompletionsRequestMessagesAssistantToolCalls(v) => v.into_mcp(),
            Response::VectorCompletionsResponse(v) => v.into_mcp(),
            Response::VectorCompletionsRequest(v) => v.into_mcp(),
            Response::FunctionsExecutionsResponse(v) => v.into_mcp(),
            Response::FunctionsExecutionsRequest(v) => v.into_mcp(),
            Response::Text(v) => v.into_mcp(),
            Response::Image(v) => v.into_mcp(),
            Response::Audio(v) => v.into_mcp(),
            Response::Video(v) => v.into_mcp(),
            Response::File(v) => v.into_mcp(),
        }
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

pub mod request_schema;


pub mod response_schema;
