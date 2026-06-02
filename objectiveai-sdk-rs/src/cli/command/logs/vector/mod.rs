pub mod completions;

#[derive(clap::Subcommand)]
pub enum Command {
    Completions {
        #[command(subcommand)]
        command: completions::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Completions(completions::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Completions(completions::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Completions { command } =>
                Ok(Request::Completions(completions::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Completions(inner) => inner.into_command(),
        }
    }
}
