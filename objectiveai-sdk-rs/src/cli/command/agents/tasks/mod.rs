//! `agents tasks` — task store + (eventual) runner.
//!
//! One leaf today: `schedule` — register a command + interval in
//! `tasks.sqlite` (#213-style write-only first slice). The runner
//! that fires schedules is follow-up work (#216).

use crate::cli::command::CommandRequest;

pub mod schedule;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Register a command + interval in `tasks.sqlite`.
    Schedule(schedule::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.agents.tasks.Request")]
pub enum Request {
    #[schemars(title = "Schedule")]
    Schedule(schedule::Request),
    #[schemars(title = "ScheduleRequestSchema")]
    ScheduleRequestSchema(schedule::request_schema::Request),
    #[schemars(title = "ScheduleResponseSchema")]
    ScheduleResponseSchema(schedule::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.agents.tasks.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Schedule")]
    Schedule(schedule::Response),
    #[schemars(title = "ScheduleRequestSchema")]
    ScheduleRequestSchema(schedule::request_schema::Response),
    #[schemars(title = "ScheduleResponseSchema")]
    ScheduleResponseSchema(schedule::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Schedule(v) => v.into_mcp(),
            ResponseItem::ScheduleRequestSchema(v) => v.into_mcp(),
            ResponseItem::ScheduleResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Schedule(cmd) => match cmd.schema {
                None => Ok(Request::Schedule(schedule::Request::try_from(cmd.args)?)),
                Some(schedule::Schema::RequestSchema(args)) => Ok(
                    Request::ScheduleRequestSchema(schedule::request_schema::Request::try_from(args)?),
                ),
                Some(schedule::Schema::ResponseSchema(args)) => Ok(
                    Request::ScheduleResponseSchema(schedule::response_schema::Request::try_from(args)?),
                ),
            },
        }
    }
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Schedule(inner) => inner.into_command(),
            Request::ScheduleRequestSchema(inner) => inner.into_command(),
            Request::ScheduleResponseSchema(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>,
    > = match request {
        Request::Schedule(req) => {
            let value = schedule::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(ResponseItem::Schedule(value))))
        }
        Request::ScheduleRequestSchema(req) => {
            let value =
                schedule::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ScheduleRequestSchema(value),
            )))
        }
        Request::ScheduleResponseSchema(req) => {
            let value =
                schedule::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ScheduleResponseSchema(value),
            )))
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
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Schedule(req) => {
            let value = schedule::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ScheduleRequestSchema(req) => {
            let value =
                schedule::request_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ScheduleResponseSchema(req) => {
            let value =
                schedule::response_schema::execute_jq(executor, req, jq, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}
