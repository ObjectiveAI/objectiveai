pub mod create;

#[derive(clap::Subcommand)]
pub enum Command {
    Create {
        #[command(subcommand)]
        command: create::Command,
    },
}

/// Single-subcommand tier: plain aliases instead of one-variant
/// wrapper enums. No aggregate schema exists at this level — schema
/// references pass straight through to the child, and the wire shape
/// is unchanged (`#[serde(untagged)]` made the wrapper invisible
/// anyway).
pub type Request = create::Request;
pub type ResponseItem = create::ResponseItem;

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Create { command } => create::Request::try_from(command),
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
    create::execute(executor, request, agent_arguments).await
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
    create::execute_jq(executor, request, jq, agent_arguments).await
}
