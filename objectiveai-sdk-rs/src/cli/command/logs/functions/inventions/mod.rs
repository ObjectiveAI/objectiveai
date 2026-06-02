pub mod recursive;
pub mod request;
pub mod response;

#[derive(clap::Subcommand)]
pub enum Command {
    Recursive {
        #[command(subcommand)]
        command: recursive::Command,
    },
    Request {
        #[command(subcommand)]
        command: request::Command,
    },
    Response {
        #[command(subcommand)]
        command: response::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Recursive(recursive::Request),
    Request(request::Request),
    Response(response::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Recursive(recursive::ResponseItem),
    Request(request::Response),
    Response(response::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Recursive { command } =>
                Ok(Request::Recursive(recursive::Request::try_from(command)?)),
            Command::Request { command } =>
                Ok(Request::Request(request::Request::try_from(command)?)),
            Command::Response { command } =>
                Ok(Request::Response(response::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Recursive(inner) => inner.into_command(),
            Request::Request(inner) => inner.into_command(),
            Request::Response(inner) => inner.into_command(),
        }
    }
}
