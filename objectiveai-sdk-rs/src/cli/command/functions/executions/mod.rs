pub mod create;

#[derive(clap::Subcommand)]
pub enum Command {
    Create {
        #[command(subcommand)]
        command: create::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Create(create::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Create(create::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Create { command } =>
                Ok(Request::Create(create::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Create(inner) => inner.into_command(),
        }
    }
}
