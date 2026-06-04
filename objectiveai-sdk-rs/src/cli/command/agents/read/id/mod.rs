//! `agents read id` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.id.Request")]
pub struct Request {
    pub path_type: Path,
    pub id: i64,
    pub jq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.read.id.Path")]
pub enum Path {
    #[serde(rename = "agents/read/id")]
    AgentsReadId,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "agents".to_string(),
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.read.id.Response")]
pub enum Response {
    // Typed log envelopes — each variant name is the PascalCase form
    // of its full leaf path under `logs/`, and the payload aliases the
    // matching `… ::get::Response` so the type stays in one place.
    #[schemars(title = "AgentsCompletionsResponse")]
    AgentsCompletionsResponse(crate::cli::command::logs::agents::completions::response::get::Response),
    #[schemars(title = "AgentsCompletionsRequest")]
    AgentsCompletionsRequest(crate::cli::command::logs::agents::completions::request::get::Response),
    #[schemars(title = "AgentsCompletionsResponseMessages")]
    AgentsCompletionsResponseMessages(crate::cli::command::logs::agents::completions::response::messages::get::Response),
    #[schemars(title = "AgentsCompletionsResponseMessagesLogprobs")]
    AgentsCompletionsResponseMessagesLogprobs(crate::cli::command::logs::agents::completions::response::messages::logprobs::get::Response),
    #[schemars(title = "AgentsCompletionsResponseMessagesToolCalls")]
    AgentsCompletionsResponseMessagesToolCalls(crate::cli::command::logs::agents::completions::response::messages::tool_calls::get::Response),

    #[schemars(title = "VectorCompletionsResponse")]
    VectorCompletionsResponse(crate::cli::command::logs::vector::completions::response::get::Response),
    #[schemars(title = "VectorCompletionsRequest")]
    VectorCompletionsRequest(crate::cli::command::logs::vector::completions::request::get::Response),

    #[schemars(title = "FunctionsExecutionsResponse")]
    FunctionsExecutionsResponse(crate::cli::command::logs::functions::executions::response::get::Response),
    #[schemars(title = "FunctionsExecutionsRequest")]
    FunctionsExecutionsRequest(crate::cli::command::logs::functions::executions::request::get::Response),

    #[schemars(title = "FunctionsInventionsResponse")]
    FunctionsInventionsResponse(crate::cli::command::logs::functions::inventions::response::get::Response),
    #[schemars(title = "FunctionsInventionsRequest")]
    FunctionsInventionsRequest(crate::cli::command::logs::functions::inventions::request::get::Response),

    #[schemars(title = "FunctionsInventionsRecursiveResponse")]
    FunctionsInventionsRecursiveResponse(crate::cli::command::logs::functions::inventions::recursive::response::get::Response),
    #[schemars(title = "FunctionsInventionsRecursiveRequest")]
    FunctionsInventionsRecursiveRequest(crate::cli::command::logs::functions::inventions::recursive::request::get::Response),

    // Collapsed text/media — one variant per content kind, regardless
    // of where the file lives. Untagged tuple variants, no wrapper keys.
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
        Ok(Self { path_type: Path::AgentsReadId,
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
            Response::AgentsCompletionsResponseMessages(v) => v.into_mcp(),
            Response::AgentsCompletionsResponseMessagesLogprobs(v) => v.into_mcp(),
            Response::AgentsCompletionsResponseMessagesToolCalls(v) => v.into_mcp(),
            Response::VectorCompletionsResponse(v) => v.into_mcp(),
            Response::VectorCompletionsRequest(v) => v.into_mcp(),
            Response::FunctionsExecutionsResponse(v) => v.into_mcp(),
            Response::FunctionsExecutionsRequest(v) => v.into_mcp(),
            Response::FunctionsInventionsResponse(v) => v.into_mcp(),
            Response::FunctionsInventionsRequest(v) => v.into_mcp(),
            Response::FunctionsInventionsRecursiveResponse(v) => v.into_mcp(),
            Response::FunctionsInventionsRecursiveRequest(v) => v.into_mcp(),
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
