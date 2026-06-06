pub mod audio;
pub mod clear;
pub mod file;
pub mod get;
pub mod image;
pub mod logprobs;
pub mod reasoning;
pub mod refusal;
pub mod subscribe;
pub mod text;
pub mod tool;
pub mod tool_calls;
pub mod video;

#[derive(clap::Subcommand)]
pub enum Command {
    Audio {
        #[command(subcommand)]
        command: audio::Command,
    },
    Clear(clear::Command),
    File {
        #[command(subcommand)]
        command: file::Command,
    },
    Get(get::Command),
    Image {
        #[command(subcommand)]
        command: image::Command,
    },
    Logprobs {
        #[command(subcommand)]
        command: logprobs::Command,
    },
    Reasoning {
        #[command(subcommand)]
        command: reasoning::Command,
    },
    Refusal {
        #[command(subcommand)]
        command: refusal::Command,
    },
    Subscribe(subscribe::Command),
    Text {
        #[command(subcommand)]
        command: text::Command,
    },
    Tool {
        #[command(subcommand)]
        command: tool::Command,
    },
    ToolCalls {
        #[command(subcommand)]
        command: tool_calls::Command,
    },
    Video {
        #[command(subcommand)]
        command: video::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.logs.agents.completions.response.messages.Request")]
pub enum Request {
    #[schemars(title = "Audio")]
    Audio(audio::Request),
    #[schemars(title = "Clear")]
    Clear(clear::Request),
    #[schemars(title = "ClearRequestSchema")]
    ClearRequestSchema(clear::request_schema::Request),
    #[schemars(title = "ClearResponseSchema")]
    ClearResponseSchema(clear::response_schema::Request),
    #[schemars(title = "File")]
    File(file::Request),
    #[schemars(title = "Get")]
    Get(get::Request),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Request),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Request),
    #[schemars(title = "Image")]
    Image(image::Request),
    #[schemars(title = "Logprobs")]
    Logprobs(logprobs::Request),
    #[schemars(title = "Reasoning")]
    Reasoning(reasoning::Request),
    #[schemars(title = "Refusal")]
    Refusal(refusal::Request),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::Request),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Request),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Request),
    #[schemars(title = "Text")]
    Text(text::Request),
    #[schemars(title = "Tool")]
    Tool(tool::Request),
    #[schemars(title = "ToolCalls")]
    ToolCalls(tool_calls::Request),
    #[schemars(title = "Video")]
    Video(video::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.logs.agents.completions.response.messages.Response")]
#[serde(untagged)]
pub enum Response {
    #[schemars(title = "Audio")]
    Audio(audio::Response),
    #[schemars(title = "Clear")]
    Clear(clear::Response),
    #[schemars(title = "ClearRequestSchema")]
    ClearRequestSchema(clear::request_schema::Response),
    #[schemars(title = "ClearResponseSchema")]
    ClearResponseSchema(clear::response_schema::Response),
    #[schemars(title = "File")]
    File(file::Response),
    #[schemars(title = "Get")]
    Get(get::Response),
    #[schemars(title = "GetRequestSchema")]
    GetRequestSchema(get::request_schema::Response),
    #[schemars(title = "GetResponseSchema")]
    GetResponseSchema(get::response_schema::Response),
    #[schemars(title = "Image")]
    Image(image::Response),
    #[schemars(title = "Logprobs")]
    Logprobs(logprobs::Response),
    #[schemars(title = "Reasoning")]
    Reasoning(reasoning::Response),
    #[schemars(title = "Refusal")]
    Refusal(refusal::Response),
    #[schemars(title = "Subscribe")]
    Subscribe(subscribe::Response),
    #[schemars(title = "SubscribeRequestSchema")]
    SubscribeRequestSchema(subscribe::request_schema::Response),
    #[schemars(title = "SubscribeResponseSchema")]
    SubscribeResponseSchema(subscribe::response_schema::Response),
    #[schemars(title = "Text")]
    Text(text::Response),
    #[schemars(title = "Tool")]
    Tool(tool::Response),
    #[schemars(title = "ToolCalls")]
    ToolCalls(tool_calls::Response),
    #[schemars(title = "Video")]
    Video(video::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Audio(v) => v.into_mcp(),
            Response::Clear(v) => v.into_mcp(),
            Response::ClearRequestSchema(v) => v.into_mcp(),
            Response::ClearResponseSchema(v) => v.into_mcp(),
            Response::File(v) => v.into_mcp(),
            Response::Get(v) => v.into_mcp(),
            Response::GetRequestSchema(v) => v.into_mcp(),
            Response::GetResponseSchema(v) => v.into_mcp(),
            Response::Image(v) => v.into_mcp(),
            Response::Logprobs(v) => v.into_mcp(),
            Response::Reasoning(v) => v.into_mcp(),
            Response::Refusal(v) => v.into_mcp(),
            Response::Subscribe(v) => v.into_mcp(),
            Response::SubscribeRequestSchema(v) => v.into_mcp(),
            Response::SubscribeResponseSchema(v) => v.into_mcp(),
            Response::Text(v) => v.into_mcp(),
            Response::Tool(v) => v.into_mcp(),
            Response::ToolCalls(v) => v.into_mcp(),
            Response::Video(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Audio { command } =>
                Ok(Request::Audio(audio::Request::try_from(command)?)),
            Command::Clear(cmd) => match cmd.schema {
                None => Ok(Request::Clear(clear::Request::try_from(cmd.args)?)),
                Some(clear::Schema::RequestSchema(args)) =>
                    Ok(Request::ClearRequestSchema(clear::request_schema::Request::try_from(args)?)),
                Some(clear::Schema::ResponseSchema(args)) =>
                    Ok(Request::ClearResponseSchema(clear::response_schema::Request::try_from(args)?)),
            },
            Command::File { command } =>
                Ok(Request::File(file::Request::try_from(command)?)),
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Image { command } =>
                Ok(Request::Image(image::Request::try_from(command)?)),
            Command::Logprobs { command } =>
                Ok(Request::Logprobs(logprobs::Request::try_from(command)?)),
            Command::Reasoning { command } =>
                Ok(Request::Reasoning(reasoning::Request::try_from(command)?)),
            Command::Refusal { command } =>
                Ok(Request::Refusal(refusal::Request::try_from(command)?)),
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) =>
                    Ok(Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?)),
                Some(subscribe::Schema::ResponseSchema(args)) =>
                    Ok(Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?)),
            },
            Command::Text { command } =>
                Ok(Request::Text(text::Request::try_from(command)?)),
            Command::Tool { command } =>
                Ok(Request::Tool(tool::Request::try_from(command)?)),
            Command::ToolCalls { command } =>
                Ok(Request::ToolCalls(tool_calls::Request::try_from(command)?)),
            Command::Video { command } =>
                Ok(Request::Video(video::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Audio(inner) => inner.into_command(),
            Request::Clear(inner) => inner.into_command(),
            Request::ClearRequestSchema(inner) => inner.into_command(),
            Request::ClearResponseSchema(inner) => inner.into_command(),
            Request::File(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Image(inner) => inner.into_command(),
            Request::Logprobs(inner) => inner.into_command(),
            Request::Reasoning(inner) => inner.into_command(),
            Request::Refusal(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
            Request::Text(inner) => inner.into_command(),
            Request::Tool(inner) => inner.into_command(),
            Request::ToolCalls(inner) => inner.into_command(),
            Request::Video(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<Response, E::Error>> + Send>> =
        match request {
            Request::Audio(req) => {
                let inner = audio::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Audio)))
            }
            Request::Clear(req) => {
                let value = clear::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Clear(value),
                )))
            }
            Request::ClearRequestSchema(req) => {
                let value = clear::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::ClearRequestSchema(value),
                )))
            }
            Request::ClearResponseSchema(req) => {
                let value = clear::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::ClearResponseSchema(value),
                )))
            }
            Request::File(req) => {
                let inner = file::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::File)))
            }
            Request::Get(req) => {
                let value = get::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Get(value),
                )))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetRequestSchema(value),
                )))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::GetResponseSchema(value),
                )))
            }
            Request::Image(req) => {
                let inner = image::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Image)))
            }
            Request::Logprobs(req) => {
                let inner = logprobs::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Logprobs)))
            }
            Request::Reasoning(req) => {
                let inner = reasoning::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Reasoning)))
            }
            Request::Refusal(req) => {
                let inner = refusal::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Refusal)))
            }
            Request::Subscribe(req) => {
                let value = subscribe::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::Subscribe(value),
                )))
            }
            Request::SubscribeRequestSchema(req) => {
                let value = subscribe::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::SubscribeRequestSchema(value),
                )))
            }
            Request::SubscribeResponseSchema(req) => {
                let value = subscribe::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    Response::SubscribeResponseSchema(value),
                )))
            }
            Request::Text(req) => {
                let inner = text::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Text)))
            }
            Request::Tool(req) => {
                let inner = tool::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Tool)))
            }
            Request::ToolCalls(req) => {
                let inner = tool_calls::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::ToolCalls)))
            }
            Request::Video(req) => {
                let inner = video::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Video)))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Audio(req) => {
                let inner = audio::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Clear(req) => {
                let value = clear::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ClearRequestSchema(req) => {
                let value = clear::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ClearResponseSchema(req) => {
                let value = clear::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::File(req) => {
                let inner = file::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Get(req) => {
                let value = get::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetRequestSchema(req) => {
                let value = get::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::GetResponseSchema(req) => {
                let value = get::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Image(req) => {
                let inner = image::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Logprobs(req) => {
                let inner = logprobs::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Reasoning(req) => {
                let inner = reasoning::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Refusal(req) => {
                let inner = refusal::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Subscribe(req) => {
                let value = subscribe::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SubscribeRequestSchema(req) => {
                let value = subscribe::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SubscribeResponseSchema(req) => {
                let value = subscribe::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Text(req) => {
                let inner = text::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Tool(req) => {
                let inner = tool::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::ToolCalls(req) => {
                let inner = tool_calls::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Video(req) => {
                let inner = video::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
