pub mod recursive;
pub mod state;

#[derive(clap::Subcommand)]
pub enum Command {
    Recursive {
        #[command(subcommand)]
        command: recursive::Command,
    },
    State {
        #[command(subcommand)]
        command: state::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Recursive(recursive::Request),
    State(state::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Recursive(recursive::ResponseItem),
    State(state::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Recursive { command } =>
                Ok(Request::Recursive(recursive::Request::try_from(command)?)),
            Command::State { command } =>
                Ok(Request::State(state::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Recursive(inner) => inner.into_command(),
            Request::State(inner) => inner.into_command(),
        }
    }
}
