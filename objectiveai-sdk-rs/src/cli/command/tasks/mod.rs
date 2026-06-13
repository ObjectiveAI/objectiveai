//! `agents tasks` — task store + (eventual) runner.
//!
//! Two leaves today:
//! - `schedule` — register a command + interval in `tasks.sqlite`.
//! - `list` — inspect rows with filters on kind / readiness /
//!   hierarchy + depth + pagination.
//!
//! The runner that fires schedules is follow-up work (#216).

use crate::cli::command::CommandRequest;

pub mod list;
pub mod run;
pub mod schedule;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Register a command + interval in `tasks.sqlite`.
    Schedule(schedule::Command),
    /// Inspect rows in `tasks.sqlite` with optional filters.
    List(list::Command),
    /// Fire every pending schedule in scope, in parallel.
    Run(run::Command),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.tasks.Request")]
pub enum Request {
    #[schemars(title = "Schedule")]
    Schedule(schedule::Request),
    #[schemars(title = "ScheduleRequestSchema")]
    ScheduleRequestSchema(schedule::request_schema::Request),
    #[schemars(title = "ScheduleResponseSchema")]
    ScheduleResponseSchema(schedule::response_schema::Request),
    #[schemars(title = "List")]
    List(list::Request),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Request),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Request),
    #[schemars(title = "Run")]
    Run(run::Request),
    #[schemars(title = "RunRequestSchema")]
    RunRequestSchema(run::request_schema::Request),
    #[schemars(title = "RunResponseSchema")]
    RunResponseSchema(run::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.tasks.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Schedule")]
    Schedule(schedule::Response),
    #[schemars(title = "ScheduleRequestSchema")]
    ScheduleRequestSchema(schedule::request_schema::Response),
    #[schemars(title = "ScheduleResponseSchema")]
    ScheduleResponseSchema(schedule::response_schema::Response),
    #[schemars(title = "List")]
    List(list::ResponseItem),
    #[schemars(title = "ListRequestSchema")]
    ListRequestSchema(list::request_schema::Response),
    #[schemars(title = "ListResponseSchema")]
    ListResponseSchema(list::response_schema::Response),
    #[schemars(title = "Run")]
    Run(run::ResponseItem),
    #[schemars(title = "RunRequestSchema")]
    RunRequestSchema(run::request_schema::Response),
    #[schemars(title = "RunResponseSchema")]
    RunResponseSchema(run::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Schedule(v) => v.into_mcp(),
            ResponseItem::ScheduleRequestSchema(v) => v.into_mcp(),
            ResponseItem::ScheduleResponseSchema(v) => v.into_mcp(),
            ResponseItem::List(v) => v.into_mcp(),
            ResponseItem::ListRequestSchema(v) => v.into_mcp(),
            ResponseItem::ListResponseSchema(v) => v.into_mcp(),
            ResponseItem::Run(v) => v.into_mcp(),
            ResponseItem::RunRequestSchema(v) => v.into_mcp(),
            ResponseItem::RunResponseSchema(v) => v.into_mcp(),
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
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) => Ok(
                    Request::ListRequestSchema(list::request_schema::Request::try_from(args)?),
                ),
                Some(list::Schema::ResponseSchema(args)) => Ok(
                    Request::ListResponseSchema(list::response_schema::Request::try_from(args)?),
                ),
            },
            Command::Run(cmd) => match cmd.schema {
                None => Ok(Request::Run(run::Request::try_from(cmd.args)?)),
                Some(run::Schema::RequestSchema(args)) => Ok(
                    Request::RunRequestSchema(run::request_schema::Request::try_from(args)?),
                ),
                Some(run::Schema::ResponseSchema(args)) => Ok(
                    Request::RunResponseSchema(run::response_schema::Request::try_from(args)?),
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
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::Run(inner) => inner.into_command(),
            Request::RunRequestSchema(inner) => inner.into_command(),
            Request::RunResponseSchema(inner) => inner.into_command(),
        }
    }

    fn request_base(&self) -> &crate::cli::command::RequestBase {
        match self {
            Request::Schedule(inner) => inner.request_base(),
            Request::ScheduleRequestSchema(inner) => inner.request_base(),
            Request::ScheduleResponseSchema(inner) => inner.request_base(),
            Request::List(inner) => inner.request_base(),
            Request::ListRequestSchema(inner) => inner.request_base(),
            Request::ListResponseSchema(inner) => inner.request_base(),
            Request::Run(inner) => inner.request_base(),
            Request::RunRequestSchema(inner) => inner.request_base(),
            Request::RunResponseSchema(inner) => inner.request_base(),
        }
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        match self {
            Request::Schedule(inner) => inner.request_base_mut(),
            Request::ScheduleRequestSchema(inner) => inner.request_base_mut(),
            Request::ScheduleResponseSchema(inner) => inner.request_base_mut(),
            Request::List(inner) => inner.request_base_mut(),
            Request::ListRequestSchema(inner) => inner.request_base_mut(),
            Request::ListResponseSchema(inner) => inner.request_base_mut(),
            Request::Run(inner) => inner.request_base_mut(),
            Request::RunRequestSchema(inner) => inner.request_base_mut(),
            Request::RunResponseSchema(inner) => inner.request_base_mut(),
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
        Request::List(req) => {
            use futures::StreamExt;
            let inner = list::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListRequestSchema(value),
            )))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::ListResponseSchema(value),
            )))
        }
        Request::Run(req) => {
            use futures::StreamExt;
            let inner = run::execute(executor, req, agent_arguments).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Run)))
        }
        Request::RunRequestSchema(req) => {
            let value =
                run::request_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::RunRequestSchema(value),
            )))
        }
        Request::RunResponseSchema(req) => {
            let value =
                run::response_schema::execute(executor, req, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(
                ResponseItem::RunResponseSchema(value),
            )))
        }
    };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,
    agent_arguments: Option<&crate::cli::command::AgentArguments>,
) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>,
    > = match request {
        Request::Schedule(req) => {
            let value = schedule::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ScheduleRequestSchema(req) => {
            let value =
                schedule::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ScheduleResponseSchema(req) => {
            let value =
                schedule::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::List(req) => {
            let inner = list::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::ListRequestSchema(req) => {
            let value =
                list::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::ListResponseSchema(req) => {
            let value =
                list::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::Run(req) => {
            let inner = run::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(inner)
        }
        Request::RunRequestSchema(req) => {
            let value =
                run::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
        Request::RunResponseSchema(req) => {
            let value =
                run::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
            Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
        }
    };
    Ok(stream)
}
