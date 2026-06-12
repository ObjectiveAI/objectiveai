//! Root-level clap `Command` plus the matching typed `Request` /
//! `ResponseItem` aggregators and conversions, mirroring the same
//! pattern every tier `mod.rs` follows for its own subcommands.

#[derive(clap::Parser)]
#[command(name = "objectiveai")]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: super::agents::Command,
    },
    Api {
        #[command(subcommand)]
        command: super::api::Command,
    },
    Db {
        #[command(subcommand)]
        command: super::db::Command,
    },
    Functions {
        #[command(subcommand)]
        command: super::functions::Command,
    },
    Mcp {
        #[command(subcommand)]
        command: super::mcp::Command,
    },
    Plugins {
        #[command(subcommand)]
        command: super::plugins::Command,
    },
    Swarms {
        #[command(subcommand)]
        command: super::swarms::Command,
    },
    Tasks {
        #[command(subcommand)]
        command: super::tasks::Command,
    },
    Tools {
        #[command(subcommand)]
        command: super::tools::Command,
    },
    Update(super::update::Command),
    Viewer {
        #[command(subcommand)]
        command: super::viewer::Command,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.Request")]
pub enum Request {
    #[schemars(title = "Agents")]
    Agents(super::agents::Request),
    #[schemars(title = "Api")]
    Api(super::api::Request),
    #[schemars(title = "Db")]
    Db(super::db::Request),
    #[schemars(title = "Functions")]
    Functions(super::functions::Request),
    #[schemars(title = "Mcp")]
    Mcp(super::mcp::Request),
    #[schemars(title = "Plugins")]
    Plugins(super::plugins::Request),
    #[schemars(title = "Swarms")]
    Swarms(super::swarms::Request),
    #[schemars(title = "Tasks")]
    Tasks(super::tasks::Request),
    #[schemars(title = "Tools")]
    Tools(super::tools::Request),
    #[schemars(title = "Update")]
    Update(super::update::Request),
    #[schemars(title = "UpdateRequestSchema")]
    UpdateRequestSchema(super::update::request_schema::Request),
    #[schemars(title = "UpdateResponseSchema")]
    UpdateResponseSchema(super::update::response_schema::Request),
    #[schemars(title = "Viewer")]
    Viewer(super::viewer::Request),
}

// Exempt from json-schema coverage: the aggregate's transitive
// expansion spans the whole command tree, which downstream
// generated TypeScript cannot emit declarations for (TS7056), and
// no consumer uses the aggregate schema — leaf schemas cover the
// wire.
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Agents")]
    Agents(super::agents::ResponseItem),
    #[schemars(title = "Api")]
    Api(super::api::Response),
    #[schemars(title = "Db")]
    Db(super::db::ResponseItem),
    #[schemars(title = "Functions")]
    Functions(super::functions::ResponseItem),
    #[schemars(title = "Mcp")]
    Mcp(super::mcp::Response),
    #[schemars(title = "Plugins")]
    Plugins(super::plugins::ResponseItem),
    #[schemars(title = "Swarms")]
    Swarms(super::swarms::ResponseItem),
    #[schemars(title = "Tasks")]
    Tasks(super::tasks::ResponseItem),
    #[schemars(title = "Tools")]
    Tools(super::tools::ResponseItem),
    #[schemars(title = "Update")]
    Update(super::update::ResponseItem),
    #[schemars(title = "UpdateRequestSchema")]
    UpdateRequestSchema(super::update::request_schema::Response),
    #[schemars(title = "UpdateResponseSchema")]
    UpdateResponseSchema(super::update::response_schema::Response),
    #[schemars(title = "Viewer")]
    Viewer(super::viewer::Response),
}

#[cfg(feature = "mcp")]
impl super::CommandResponse for ResponseItem {
    fn into_mcp(self) -> super::McpResponseItem {
        match self {
            ResponseItem::Agents(v) => v.into_mcp(),
            ResponseItem::Api(v) => v.into_mcp(),
            ResponseItem::Db(v) => v.into_mcp(),
            ResponseItem::Functions(v) => v.into_mcp(),
            ResponseItem::Mcp(v) => v.into_mcp(),
            ResponseItem::Plugins(v) => v.into_mcp(),
            ResponseItem::Swarms(v) => v.into_mcp(),
            ResponseItem::Tasks(v) => v.into_mcp(),
            ResponseItem::Tools(v) => v.into_mcp(),
            ResponseItem::Update(v) => v.into_mcp(),
            ResponseItem::UpdateRequestSchema(v) => v.into_mcp(),
            ResponseItem::UpdateResponseSchema(v) => v.into_mcp(),
            ResponseItem::Viewer(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = super::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Agents { command } =>
                Ok(Request::Agents(super::agents::Request::try_from(command)?)),
            Command::Api { command } =>
                Ok(Request::Api(super::api::Request::try_from(command)?)),
            Command::Db { command } =>
                Ok(Request::Db(super::db::Request::try_from(command)?)),
            Command::Functions { command } =>
                Ok(Request::Functions(super::functions::Request::try_from(command)?)),
            Command::Mcp { command } =>
                Ok(Request::Mcp(super::mcp::Request::try_from(command)?)),
            Command::Plugins { command } =>
                Ok(Request::Plugins(super::plugins::Request::try_from(command)?)),
            Command::Swarms { command } =>
                Ok(Request::Swarms(super::swarms::Request::try_from(command)?)),
            Command::Tasks { command } =>
                Ok(Request::Tasks(super::tasks::Request::try_from(command)?)),
            Command::Tools { command } =>
                Ok(Request::Tools(super::tools::Request::try_from(command)?)),
            Command::Update(cmd) => match cmd.schema {
                None => Ok(Request::Update(super::update::Request::try_from(cmd.args)?)),
                Some(super::update::Schema::RequestSchema(args)) =>
                    Ok(Request::UpdateRequestSchema(super::update::request_schema::Request::try_from(args)?)),
                Some(super::update::Schema::ResponseSchema(args)) =>
                    Ok(Request::UpdateResponseSchema(super::update::response_schema::Request::try_from(args)?)),
            },
            Command::Viewer { command } =>
                Ok(Request::Viewer(super::viewer::Request::try_from(command)?)),
        }
    }
}

impl super::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Agents(inner) => inner.into_command(),
            Request::Api(inner) => inner.into_command(),
            Request::Db(inner) => inner.into_command(),
            Request::Functions(inner) => inner.into_command(),
            Request::Mcp(inner) => inner.into_command(),
            Request::Plugins(inner) => inner.into_command(),
            Request::Swarms(inner) => inner.into_command(),
            Request::Tasks(inner) => inner.into_command(),
            Request::Tools(inner) => inner.into_command(),
            Request::Update(inner) => inner.into_command(),
            Request::UpdateRequestSchema(inner) => inner.into_command(),
            Request::UpdateResponseSchema(inner) => inner.into_command(),
            Request::Viewer(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: super::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>> =
        match request {
            Request::Agents(req) => {
                let inner = super::agents::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
            }
            Request::Api(req) => {
                let inner = super::api::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Api)))
            }
            Request::Db(req) => {
                let inner = super::db::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Db)))
            }
            Request::Functions(req) => {
                let inner = super::functions::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
            }
            Request::Mcp(req) => {
                let inner = super::mcp::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
            }
            Request::Plugins(req) => {
                let inner = super::plugins::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Plugins)))
            }
            Request::Swarms(req) => {
                let inner = super::swarms::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Swarms)))
            }
            Request::Tasks(req) => {
                let inner = super::tasks::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Tasks)))
            }
            Request::Tools(req) => {
                let inner = super::tools::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Tools)))
            }
            Request::Update(req) => {
                let inner = super::update::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Update)))
            }
            Request::UpdateRequestSchema(req) => {
                let value = super::update::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(ResponseItem::UpdateRequestSchema(value))))
            }
            Request::UpdateResponseSchema(req) => {
                let value = super::update::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(ResponseItem::UpdateResponseSchema(value))))
            }
            Request::Viewer(req) => {
                let inner = super::viewer::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: super::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Agents(req) => {
                let inner = super::agents::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Api(req) => {
                let inner = super::api::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Db(req) => {
                let inner = super::db::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Functions(req) => {
                let inner = super::functions::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Mcp(req) => {
                let inner = super::mcp::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Plugins(req) => {
                let inner = super::plugins::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Swarms(req) => {
                let inner = super::swarms::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Tasks(req) => {
                let inner = super::tasks::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Tools(req) => {
                let inner = super::tools::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::Update(req) => {
                let inner = super::update::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
            Request::UpdateRequestSchema(req) => {
                let value = super::update::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(value)))
            }
            Request::UpdateResponseSchema(req) => {
                let value = super::update::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(value)))
            }
            Request::Viewer(req) => {
                let inner = super::viewer::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}

/// Parse an argv slice into a typed [`Request`]. Accepts either
/// shape:
///
/// - With a program-name prefix (`["objectiveai", "agents", "list"]`)
///   — matches `std::env::args()` and binary entry-point usage.
/// - Without one (`["agents", "list"]`) — matches
///   [`super::CommandRequest::into_command`]'s output shape and the
///   "command-only" shape MCP callers send.
///
/// If `args[0]` is `"objectiveai"` we pass it straight through;
/// otherwise we prepend so clap (which always treats argv[0] as the
/// program name) sees a well-formed argv. Hides clap behind the SDK
/// boundary so downstream crates can dispatch arbitrary argv without
/// taking a clap dep themselves.
pub fn parse_request(args: &[String]) -> Result<Request, ParseError> {
    let command = if args.first().map(String::as_str) == Some("objectiveai") {
        <Command as clap::Parser>::try_parse_from(args)?
    } else {
        let argv = std::iter::once("objectiveai".to_string())
            .chain(args.iter().cloned());
        <Command as clap::Parser>::try_parse_from(argv)?
    };
    Ok(Request::try_from(command)?)
}

/// Error from [`parse_request`]. Either clap rejected the argv
/// ([`ParseError::Clap`] — `--help`, unknown subcommand, missing
/// required arg, etc.) or the typed `Command` couldn't be lowered
/// into a `Request` ([`ParseError::FromArgs`] — inline body JSON
/// failed to parse, etc.).
#[derive(Debug)]
pub enum ParseError {
    Clap(clap::Error),
    FromArgs(super::FromArgsError),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Clap(e) => write!(f, "{e}"),
            ParseError::FromArgs(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<clap::Error> for ParseError {
    fn from(e: clap::Error) -> Self {
        ParseError::Clap(e)
    }
}

impl From<super::FromArgsError> for ParseError {
    fn from(e: super::FromArgsError) -> Self {
        ParseError::FromArgs(e)
    }
}
