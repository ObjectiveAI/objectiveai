//! Root-level clap `Command` plus the matching typed `Request` /
//! `ResponseItem` aggregators and conversions, mirroring the same
//! pattern every tier `mod.rs` follows for its own subcommands.

#[derive(clap::Subcommand)]
pub enum Subcommand {
    Agents {
        #[command(subcommand)]
        command: super::agents::Command,
    },
    Api {
        #[command(subcommand)]
        command: super::api::Command,
    },
    Daemon {
        #[command(subcommand)]
        command: super::daemon::Command,
    },
    Db {
        #[command(subcommand)]
        command: super::db::Command,
    },
    Functions {
        #[command(subcommand)]
        command: super::functions::Command,
    },
    Laboratories {
        #[command(subcommand)]
        command: super::laboratories::Command,
    },
    Mcp {
        #[command(subcommand)]
        command: super::mcp::Command,
    },
    Plugins {
        #[command(subcommand)]
        command: super::plugins::Command,
    },
    /// Run a Python snippet and return its output as JSON.
    Python(super::python::Command),
    Swarms {
        #[command(subcommand)]
        command: super::swarms::Command,
    },
    Update(super::update::Command),
    Channels {
        #[command(subcommand)]
        command: super::channels::Command,
    },
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
    #[schemars(title = "Daemon")]
    Daemon(super::daemon::Request),
    #[schemars(title = "Db")]
    Db(super::db::Request),
    #[schemars(title = "Functions")]
    Functions(super::functions::Request),
    #[schemars(title = "Laboratories")]
    Laboratories(super::laboratories::Request),
    #[schemars(title = "Mcp")]
    Mcp(super::mcp::Request),
    #[schemars(title = "Plugins")]
    Plugins(super::plugins::Request),
    #[schemars(title = "Python")]
    Python(super::python::Request),
    #[schemars(title = "PythonRequestSchema")]
    PythonRequestSchema(super::python::request_schema::Request),
    #[schemars(title = "PythonResponseSchema")]
    PythonResponseSchema(super::python::response_schema::Request),
    #[schemars(title = "Swarms")]
    Swarms(super::swarms::Request),
    #[schemars(title = "Update")]
    Update(super::update::Request),
    #[schemars(title = "UpdateRequestSchema")]
    UpdateRequestSchema(super::update::request_schema::Request),
    #[schemars(title = "UpdateResponseSchema")]
    UpdateResponseSchema(super::update::response_schema::Request),
    #[schemars(title = "Channels")]
    Channels(super::channels::Request),
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
    #[schemars(title = "Daemon")]
    Daemon(super::daemon::ResponseItem),
    #[schemars(title = "Db")]
    Db(super::db::ResponseItem),
    #[schemars(title = "Functions")]
    Functions(super::functions::ResponseItem),
    #[schemars(title = "Laboratories")]
    Laboratories(super::laboratories::ResponseItem),
    #[schemars(title = "Mcp")]
    Mcp(super::mcp::Response),
    #[schemars(title = "Plugins")]
    Plugins(super::plugins::ResponseItem),
    #[schemars(title = "Python")]
    Python(serde_json::Value),
    #[schemars(title = "PythonRequestSchema")]
    PythonRequestSchema(super::python::request_schema::Response),
    #[schemars(title = "PythonResponseSchema")]
    PythonResponseSchema(super::python::response_schema::Response),
    #[schemars(title = "Swarms")]
    Swarms(super::swarms::ResponseItem),
    #[schemars(title = "Update")]
    Update(super::update::ResponseItem),
    #[schemars(title = "UpdateRequestSchema")]
    UpdateRequestSchema(super::update::request_schema::Response),
    #[schemars(title = "UpdateResponseSchema")]
    UpdateResponseSchema(super::update::response_schema::Response),
    #[schemars(title = "Channels")]
    Channels(super::channels::ResponseItem),
    #[schemars(title = "Viewer")]
    Viewer(super::viewer::Response),
}

#[cfg(feature = "mcp")]
impl super::CommandResponse for ResponseItem {
    fn into_mcp(self) -> super::McpResponseItem {
        match self {
            ResponseItem::Agents(v) => v.into_mcp(),
            ResponseItem::Api(v) => v.into_mcp(),
            ResponseItem::Daemon(v) => v.into_mcp(),
            ResponseItem::Db(v) => v.into_mcp(),
            ResponseItem::Functions(v) => v.into_mcp(),
            ResponseItem::Laboratories(v) => v.into_mcp(),
            ResponseItem::Mcp(v) => v.into_mcp(),
            ResponseItem::Plugins(v) => v.into_mcp(),
            ResponseItem::Python(v) => v.into_mcp(),
            ResponseItem::PythonRequestSchema(v) => v.into_mcp(),
            ResponseItem::PythonResponseSchema(v) => v.into_mcp(),
            ResponseItem::Swarms(v) => v.into_mcp(),
            ResponseItem::Update(v) => v.into_mcp(),
            ResponseItem::UpdateRequestSchema(v) => v.into_mcp(),
            ResponseItem::UpdateResponseSchema(v) => v.into_mcp(),
            ResponseItem::Channels(v) => v.into_mcp(),
            ResponseItem::Viewer(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Subcommand> for Request {
    type Error = super::FromArgsError;
    fn try_from(command: Subcommand) -> Result<Self, Self::Error> {
        match command {
            Subcommand::Agents { command } =>
                Ok(Request::Agents(super::agents::Request::try_from(command)?)),
            Subcommand::Api { command } =>
                Ok(Request::Api(super::api::Request::try_from(command)?)),
            Subcommand::Daemon { command } =>
                Ok(Request::Daemon(super::daemon::Request::try_from(command)?)),
            Subcommand::Db { command } =>
                Ok(Request::Db(super::db::Request::try_from(command)?)),
            Subcommand::Functions { command } =>
                Ok(Request::Functions(super::functions::Request::try_from(command)?)),
            Subcommand::Laboratories { command } =>
                Ok(Request::Laboratories(super::laboratories::Request::try_from(command)?)),
            Subcommand::Mcp { command } =>
                Ok(Request::Mcp(super::mcp::Request::try_from(command)?)),
            Subcommand::Plugins { command } =>
                Ok(Request::Plugins(super::plugins::Request::try_from(command)?)),
            Subcommand::Python(cmd) => match cmd.schema {
                None => Ok(Request::Python(super::python::Request::try_from(cmd.args)?)),
                Some(super::python::Schema::RequestSchema(args)) =>
                    Ok(Request::PythonRequestSchema(super::python::request_schema::Request::try_from(args)?)),
                Some(super::python::Schema::ResponseSchema(args)) =>
                    Ok(Request::PythonResponseSchema(super::python::response_schema::Request::try_from(args)?)),
            },
            Subcommand::Swarms { command } =>
                Ok(Request::Swarms(super::swarms::Request::try_from(command)?)),
            Subcommand::Update(cmd) => match cmd.schema {
                None => Ok(Request::Update(super::update::Request::try_from(cmd.args)?)),
                Some(super::update::Schema::RequestSchema(args)) =>
                    Ok(Request::UpdateRequestSchema(super::update::request_schema::Request::try_from(args)?)),
                Some(super::update::Schema::ResponseSchema(args)) =>
                    Ok(Request::UpdateResponseSchema(super::update::response_schema::Request::try_from(args)?)),
            },
            Subcommand::Channels { command } =>
                Ok(Request::Channels(super::channels::Request::try_from(command)?)),
            Subcommand::Viewer { command } =>
                Ok(Request::Viewer(super::viewer::Request::try_from(command)?)),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = super::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match (command.request, command.command) {
            // `--request <json>`: deserialize straight into the aggregate
            // Request (serde-tagged on `path_type`) — the same wire shape the
            // SDKs already serialize. Mutually exclusive with a subcommand
            // (clap enforces it), so the `_` arm can't carry a command.
            (Some(json), _) => {
                let de = &mut serde_json::Deserializer::from_str(&json);
                serde_path_to_error::deserialize(de)
                    .map_err(|e| super::FromArgsError::json("request", e))
            }
            (None, Some(subcommand)) => Request::try_from(subcommand),
            // Unreachable in practice: `arg_required_else_help` turns a bare
            // invocation into a clap help error before conversion.
            (None, None) => Err(super::FromArgsError::path_parse(
                "command",
                "a command path or --request is required".to_string(),
            )),
        }
    }
}

impl super::CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Agents(inner) => inner.request_base(),
            Request::Api(inner) => inner.request_base(),
            Request::Daemon(inner) => inner.request_base(),
            Request::Db(inner) => inner.request_base(),
            Request::Functions(inner) => inner.request_base(),
            Request::Laboratories(inner) => inner.request_base(),
            Request::Mcp(inner) => inner.request_base(),
            Request::Plugins(inner) => inner.request_base(),
            Request::Python(inner) => inner.request_base(),
            Request::PythonRequestSchema(inner) => inner.request_base(),
            Request::PythonResponseSchema(inner) => inner.request_base(),
            Request::Swarms(inner) => inner.request_base(),
            Request::Update(inner) => inner.request_base(),
            Request::UpdateRequestSchema(inner) => inner.request_base(),
            Request::UpdateResponseSchema(inner) => inner.request_base(),
            Request::Channels(inner) => inner.request_base(),
            Request::Viewer(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Agents(inner) => inner.request_base_mut(),
            Request::Api(inner) => inner.request_base_mut(),
            Request::Daemon(inner) => inner.request_base_mut(),
            Request::Db(inner) => inner.request_base_mut(),
            Request::Functions(inner) => inner.request_base_mut(),
            Request::Laboratories(inner) => inner.request_base_mut(),
            Request::Mcp(inner) => inner.request_base_mut(),
            Request::Plugins(inner) => inner.request_base_mut(),
            Request::Python(inner) => inner.request_base_mut(),
            Request::PythonRequestSchema(inner) => inner.request_base_mut(),
            Request::PythonResponseSchema(inner) => inner.request_base_mut(),
            Request::Swarms(inner) => inner.request_base_mut(),
            Request::Update(inner) => inner.request_base_mut(),
            Request::UpdateRequestSchema(inner) => inner.request_base_mut(),
            Request::UpdateResponseSchema(inner) => inner.request_base_mut(),
            Request::Channels(inner) => inner.request_base_mut(),
            Request::Viewer(inner) => inner.request_base_mut(),
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
            Request::Daemon(req) => {
                let inner = super::daemon::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Daemon)))
            }
            Request::Db(req) => {
                let inner = super::db::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Db)))
            }
            Request::Functions(req) => {
                let inner = super::functions::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
            }
            Request::Laboratories(req) => {
                let inner = super::laboratories::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Laboratories)))
            }
            Request::Mcp(req) => {
                let inner = super::mcp::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
            }
            Request::Plugins(req) => {
                let inner = super::plugins::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Plugins)))
            }
            Request::Python(req) => {
                let value = super::python::execute(executor, req, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(ResponseItem::Python(value))))
            }
            Request::PythonRequestSchema(req) => {
                let value = super::python::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(ResponseItem::PythonRequestSchema(value))))
            }
            Request::PythonResponseSchema(req) => {
                let value = super::python::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(ResponseItem::PythonResponseSchema(value))))
            }
            Request::Swarms(req) => {
                let inner = super::swarms::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Swarms)))
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
            Request::Channels(req) => {
                let inner = super::channels::execute(executor, req, agent_arguments).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Channels)))
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
            Request::Daemon(req) => {
                let inner = super::daemon::execute_transform(executor, req, transform, agent_arguments).await?;
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
            Request::Laboratories(req) => {
                let inner = super::laboratories::execute_transform(executor, req, transform, agent_arguments).await?;
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
            Request::Python(req) => {
                let value = super::python::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(value)))
            }
            Request::PythonRequestSchema(req) => {
                let value = super::python::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(value)))
            }
            Request::PythonResponseSchema(req) => {
                let value = super::python::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(super::StreamOnce::new(Ok(value)))
            }
            Request::Swarms(req) => {
                let inner = super::swarms::execute_transform(executor, req, transform, agent_arguments).await?;
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
            Request::Channels(req) => {
                let inner = super::channels::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(inner)
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

/// Top-level CLI parser: a command path, OR `--request <json>` to execute a
/// JSON `CliCommandRequest` directly. The two are mutually exclusive
/// (`args_conflicts_with_subcommands`, enforced natively by clap); a bare
/// invocation prints help (`arg_required_else_help`). `--request` is a
/// legitimate but unadvertised programmatic entry point.
#[derive(clap::Parser)]
#[command(
    name = "objectiveai",
    version,
    args_conflicts_with_subcommands = true,
    arg_required_else_help = true
)]
pub struct Command {
    /// Execute this JSON `CliCommandRequest` instead of a command path.
    /// Mutually exclusive with any command.
    #[arg(long, value_name = "JSON")]
    pub request: Option<String>,
    #[command(subcommand)]
    pub command: Option<Subcommand>,
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

/// `/listen` mirror of [`Request`] — the root of the distributed
/// `ListenerExecution` tree ([`crate::cli::broadcast_listener`]'s
/// stream item): one variant per child wrapping its
/// `ListenerExecution`. The broadcast always carries the typed
/// PRE-transform items (the producer tee sits below the CLI's
/// jq/python transform), so every execution lands on its typed
/// variant. Executions whose `path_type` this build's types predate
/// are skipped by the listener, so there is no unknown fallback.
#[cfg(feature = "cli-listener")]
pub enum ListenerExecution {
    Agents(super::agents::ListenerExecution),
    Api(super::api::ListenerExecution),
    Daemon(super::daemon::ListenerExecution),
    Db(super::db::ListenerExecution),
    Functions(super::functions::ListenerExecution),
    Laboratories(super::laboratories::ListenerExecution),
    Mcp(super::mcp::ListenerExecution),
    Plugins(super::plugins::ListenerExecution),
    Python(super::python::ListenerExecution),
    PythonRequestSchema(super::python::request_schema::ListenerExecution),
    PythonResponseSchema(super::python::response_schema::ListenerExecution),
    Swarms(super::swarms::ListenerExecution),
    Update(super::update::ListenerExecution),
    UpdateRequestSchema(super::update::request_schema::ListenerExecution),
    UpdateResponseSchema(super::update::response_schema::ListenerExecution),
    Channels(super::channels::ListenerExecution),
    Viewer(super::viewer::ListenerExecution),
}
