pub mod standard;
pub mod swiss_system;

#[derive(clap::Subcommand)]
pub enum Command {
    Standard(standard::Command),
    SwissSystem(swiss_system::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Standard(standard::Request),
    StandardRequestSchema(standard::request_schema::Request),
    StandardResponseSchema(standard::response_schema::Request),
    SwissSystem(swiss_system::Request),
    SwissSystemRequestSchema(swiss_system::request_schema::Request),
    SwissSystemResponseSchema(swiss_system::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Standard(standard::ResponseItem),
    StandardRequestSchema(standard::request_schema::Response),
    StandardResponseSchema(standard::response_schema::Response),
    SwissSystem(swiss_system::ResponseItem),
    SwissSystemRequestSchema(swiss_system::request_schema::Response),
    SwissSystemResponseSchema(swiss_system::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Standard(cmd) => match cmd.schema {
                None => Ok(Request::Standard(standard::Request::try_from(cmd.args)?)),
                Some(standard::Schema::RequestSchema(args)) =>
                    Ok(Request::StandardRequestSchema(standard::request_schema::Request::try_from(args)?)),
                Some(standard::Schema::ResponseSchema(args)) =>
                    Ok(Request::StandardResponseSchema(standard::response_schema::Request::try_from(args)?)),
            },
            Command::SwissSystem(cmd) => match cmd.schema {
                None => Ok(Request::SwissSystem(swiss_system::Request::try_from(cmd.args)?)),
                Some(swiss_system::Schema::RequestSchema(args)) =>
                    Ok(Request::SwissSystemRequestSchema(swiss_system::request_schema::Request::try_from(args)?)),
                Some(swiss_system::Schema::ResponseSchema(args)) =>
                    Ok(Request::SwissSystemResponseSchema(swiss_system::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Standard(inner) => inner.into_command(),
            Request::StandardRequestSchema(inner) => inner.into_command(),
            Request::StandardResponseSchema(inner) => inner.into_command(),
            Request::SwissSystem(inner) => inner.into_command(),
            Request::SwissSystemRequestSchema(inner) => inner.into_command(),
            Request::SwissSystemResponseSchema(inner) => inner.into_command(),
        }
    }
}
