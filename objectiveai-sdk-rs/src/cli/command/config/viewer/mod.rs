pub mod address;
pub mod get;
pub mod port;
pub mod secret;
pub mod signature;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Address {
        #[command(subcommand)]
        command: address::Command,
    },
    Port {
        #[command(subcommand)]
        command: port::Command,
    },
    Secret {
        #[command(subcommand)]
        command: secret::Command,
    },
    Signature {
        #[command(subcommand)]
        command: signature::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Address(address::Request),
    Port(port::Request),
    Secret(secret::Request),
    Signature(signature::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Address(address::Response),
    Port(port::Response),
    Secret(secret::Response),
    Signature(signature::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Address { command } =>
                Ok(Request::Address(address::Request::try_from(command)?)),
            Command::Port { command } =>
                Ok(Request::Port(port::Request::try_from(command)?)),
            Command::Secret { command } =>
                Ok(Request::Secret(secret::Request::try_from(command)?)),
            Command::Signature { command } =>
                Ok(Request::Signature(signature::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Address(inner) => inner.into_command(),
            Request::Port(inner) => inner.into_command(),
            Request::Secret(inner) => inner.into_command(),
            Request::Signature(inner) => inner.into_command(),
        }
    }
}
