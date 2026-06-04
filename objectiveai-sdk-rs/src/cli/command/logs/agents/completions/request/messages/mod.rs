pub mod audio;
pub mod file;
pub mod image;
pub mod text;
pub mod video;

#[derive(clap::Subcommand)]
pub enum Command {
    Audio {
        #[command(subcommand)]
        command: audio::Command,
    },
    File {
        #[command(subcommand)]
        command: file::Command,
    },
    Image {
        #[command(subcommand)]
        command: image::Command,
    },
    Text {
        #[command(subcommand)]
        command: text::Command,
    },
    Video {
        #[command(subcommand)]
        command: video::Command,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.logs.agents.completions.request.messages.Request")]
pub enum Request {
    #[schemars(title = "Audio")]
    Audio(audio::Request),
    #[schemars(title = "File")]
    File(file::Request),
    #[schemars(title = "Image")]
    Image(image::Request),
    #[schemars(title = "Text")]
    Text(text::Request),
    #[schemars(title = "Video")]
    Video(video::Request),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.logs.agents.completions.request.messages.Response")]
pub enum Response {
    #[schemars(title = "Audio")]
    Audio(audio::Response),
    #[schemars(title = "File")]
    File(file::Response),
    #[schemars(title = "Image")]
    Image(image::Response),
    #[schemars(title = "Text")]
    Text(text::Response),
    #[schemars(title = "Video")]
    Video(video::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for Response {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            Response::Audio(v) => v.into_mcp(),
            Response::File(v) => v.into_mcp(),
            Response::Image(v) => v.into_mcp(),
            Response::Text(v) => v.into_mcp(),
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
            Command::File { command } =>
                Ok(Request::File(file::Request::try_from(command)?)),
            Command::Image { command } =>
                Ok(Request::Image(image::Request::try_from(command)?)),
            Command::Text { command } =>
                Ok(Request::Text(text::Request::try_from(command)?)),
            Command::Video { command } =>
                Ok(Request::Video(video::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Audio(inner) => inner.into_command(),
            Request::File(inner) => inner.into_command(),
            Request::Image(inner) => inner.into_command(),
            Request::Text(inner) => inner.into_command(),
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
            Request::File(req) => {
                let inner = file::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::File)))
            }
            Request::Image(req) => {
                let inner = image::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Image)))
            }
            Request::Text(req) => {
                let inner = text::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(Response::Text)))
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
            Request::File(req) => {
                let inner = file::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Image(req) => {
                let inner = image::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Text(req) => {
                let inner = text::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Video(req) => {
                let inner = video::execute_jq(executor, req, jq, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
