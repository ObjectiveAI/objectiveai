pub mod active;
pub mod available;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Direct children of the calling agent.
    Active(active::Command),
    /// Remote agents available from a given source.
    Available(available::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Active(active::Request),
    ActiveRequestSchema(active::request_schema::Request),
    ActiveResponseSchema(active::response_schema::Request),
    Available(available::Request),
    AvailableRequestSchema(available::request_schema::Request),
    AvailableResponseSchema(available::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Active(active::ResponseItem),
    ActiveRequestSchema(active::request_schema::Response),
    ActiveResponseSchema(active::response_schema::Response),
    Available(available::ResponseItem),
    AvailableRequestSchema(available::request_schema::Response),
    AvailableResponseSchema(available::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Active(cmd) => match cmd.schema {
                None => Ok(Request::Active(active::Request::try_from(cmd.args)?)),
                Some(active::Schema::RequestSchema(args)) =>
                    Ok(Request::ActiveRequestSchema(active::request_schema::Request::try_from(args)?)),
                Some(active::Schema::ResponseSchema(args)) =>
                    Ok(Request::ActiveResponseSchema(active::response_schema::Request::try_from(args)?)),
            },
            Command::Available(cmd) => match cmd.schema {
                None => Ok(Request::Available(available::Request::try_from(cmd.args)?)),
                Some(available::Schema::RequestSchema(args)) =>
                    Ok(Request::AvailableRequestSchema(available::request_schema::Request::try_from(args)?)),
                Some(available::Schema::ResponseSchema(args)) =>
                    Ok(Request::AvailableResponseSchema(available::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Active(inner) => inner.into_command(),
            Request::ActiveRequestSchema(inner) => inner.into_command(),
            Request::ActiveResponseSchema(inner) => inner.into_command(),
            Request::Available(inner) => inner.into_command(),
            Request::AvailableRequestSchema(inner) => inner.into_command(),
            Request::AvailableResponseSchema(inner) => inner.into_command(),
        }
    }
}
