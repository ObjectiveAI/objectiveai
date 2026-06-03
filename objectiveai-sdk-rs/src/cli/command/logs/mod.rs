pub mod agents;
pub mod clear;
pub mod functions;
pub mod vector;

#[derive(clap::Subcommand)]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: agents::Command,
    },
    Clear(clear::Command),
    Functions {
        #[command(subcommand)]
        command: functions::Command,
    },
    Vector {
        #[command(subcommand)]
        command: vector::Command,
    },
}

#[derive(Debug, Clone)]
pub enum Request {
    Agents(agents::Request),
    Clear(clear::Request),
    ClearRequestSchema(clear::request_schema::Request),
    ClearResponseSchema(clear::response_schema::Request),
    Functions(functions::Request),
    Vector(vector::Request),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ResponseItem {
    Agents(agents::ResponseItem),
    Clear(clear::Response),
    ClearRequestSchema(clear::request_schema::Response),
    ClearResponseSchema(clear::response_schema::Response),
    Functions(functions::ResponseItem),
    Vector(vector::ResponseItem),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Agents(v) => v.into_mcp(),
            ResponseItem::Clear(v) => v.into_mcp(),
            ResponseItem::ClearRequestSchema(v) => v.into_mcp(),
            ResponseItem::ClearResponseSchema(v) => v.into_mcp(),
            ResponseItem::Functions(v) => v.into_mcp(),
            ResponseItem::Vector(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Agents { command } =>
                Ok(Request::Agents(agents::Request::try_from(command)?)),
            Command::Clear(cmd) => match cmd.schema {
                None => Ok(Request::Clear(clear::Request::try_from(cmd.args)?)),
                Some(clear::Schema::RequestSchema(args)) =>
                    Ok(Request::ClearRequestSchema(clear::request_schema::Request::try_from(args)?)),
                Some(clear::Schema::ResponseSchema(args)) =>
                    Ok(Request::ClearResponseSchema(clear::response_schema::Request::try_from(args)?)),
            },
            Command::Functions { command } =>
                Ok(Request::Functions(functions::Request::try_from(command)?)),
            Command::Vector { command } =>
                Ok(Request::Vector(vector::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Agents(inner) => inner.into_command(),
            Request::Clear(inner) => inner.into_command(),
            Request::ClearRequestSchema(inner) => inner.into_command(),
            Request::ClearResponseSchema(inner) => inner.into_command(),
            Request::Functions(inner) => inner.into_command(),
            Request::Vector(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>> =
        match request {
            Request::Agents(req) => {
                let inner = agents::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
            }
            Request::Clear(req) => {
                let value = clear::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::Clear(value),
                )))
            }
            Request::ClearRequestSchema(req) => {
                let value = clear::request_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ClearRequestSchema(value),
                )))
            }
            Request::ClearResponseSchema(req) => {
                let value = clear::response_schema::execute(executor, req).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::ClearResponseSchema(value),
                )))
            }
            Request::Functions(req) => {
                let inner = functions::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
            }
            Request::Vector(req) => {
                let inner = vector::execute(executor, req).await?;
                Box::pin(inner.map(|r| r.map(ResponseItem::Vector)))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    jq: String,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Agents(req) => {
                let inner = agents::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Clear(req) => {
                let value = clear::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ClearRequestSchema(req) => {
                let value = clear::request_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::ClearResponseSchema(req) => {
                let value = clear::response_schema::execute_jq(executor, req, jq).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::Functions(req) => {
                let inner = functions::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
            Request::Vector(req) => {
                let inner = vector::execute_jq(executor, req, jq).await?;
                Box::pin(inner)
            }
        };
    Ok(stream)
}
