pub mod kill;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    Kill(kill::Command),
    Spawn(spawn::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Kill(kill::Request),
    KillRequestSchema(kill::request_schema::Request),
    KillResponseSchema(kill::response_schema::Request),
    Spawn(spawn::Request),
    SpawnRequestSchema(spawn::request_schema::Request),
    SpawnResponseSchema(spawn::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Kill(kill::Response),
    KillRequestSchema(kill::request_schema::Response),
    KillResponseSchema(kill::response_schema::Response),
    Spawn(spawn::Response),
    SpawnRequestSchema(spawn::request_schema::Response),
    SpawnResponseSchema(spawn::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Kill(cmd) => match cmd.schema {
                None => Ok(Request::Kill(kill::Request::try_from(cmd.args)?)),
                Some(kill::Schema::RequestSchema(args)) =>
                    Ok(Request::KillRequestSchema(kill::request_schema::Request::try_from(args)?)),
                Some(kill::Schema::ResponseSchema(args)) =>
                    Ok(Request::KillResponseSchema(kill::response_schema::Request::try_from(args)?)),
            },
            Command::Spawn(cmd) => match cmd.schema {
                None => Ok(Request::Spawn(spawn::Request::try_from(cmd.args)?)),
                Some(spawn::Schema::RequestSchema(args)) =>
                    Ok(Request::SpawnRequestSchema(spawn::request_schema::Request::try_from(args)?)),
                Some(spawn::Schema::ResponseSchema(args)) =>
                    Ok(Request::SpawnResponseSchema(spawn::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Kill(inner) => inner.into_command(),
            Request::KillRequestSchema(inner) => inner.into_command(),
            Request::KillResponseSchema(inner) => inner.into_command(),
            Request::Spawn(inner) => inner.into_command(),
            Request::SpawnRequestSchema(inner) => inner.into_command(),
            Request::SpawnResponseSchema(inner) => inner.into_command(),
        }
    }
}
