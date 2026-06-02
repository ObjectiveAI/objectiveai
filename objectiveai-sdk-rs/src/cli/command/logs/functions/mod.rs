pub mod executions;
pub mod inventions;

#[derive(clap::Subcommand)]
pub enum Command {
    Executions {
        #[command(subcommand)]
        command: executions::Command,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Executions(executions::Request),
    Inventions(inventions::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Executions(executions::ResponseItem),
    Inventions(inventions::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Executions { command } =>
                Ok(Request::Executions(executions::Request::try_from(command)?)),
            Command::Inventions { command } =>
                Ok(Request::Inventions(inventions::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Executions(inner) => inner.into_command(),
            Request::Inventions(inner) => inner.into_command(),
        }
    }
}
