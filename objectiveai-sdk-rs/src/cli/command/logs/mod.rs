pub mod agents;
pub mod clear;
pub mod functions;
pub mod vector;

#[derive(clap::Subcommand)]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: agents::Command,
    },
    Clear(clear::Command),
    Functions {
        #[command(subcommand)]
        command: functions::Command,
    },
    Vector {
        #[command(subcommand)]
        command: vector::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Agents(agents::Request),
    Clear(clear::Request),
    ClearRequestSchema(clear::request_schema::Request),
    ClearResponseSchema(clear::response_schema::Request),
    Functions(functions::Request),
    Vector(vector::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Agents(agents::ResponseItem),
    Clear(clear::Response),
    ClearRequestSchema(clear::request_schema::Response),
    ClearResponseSchema(clear::response_schema::Response),
    Functions(functions::ResponseItem),
    Vector(vector::ResponseItem),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Agents { command } =>
                Ok(Request::Agents(agents::Request::try_from(command)?)),
            Command::Clear(cmd) => match cmd.schema {
                None => Ok(Request::Clear(clear::Request::try_from(cmd.args)?)),
                Some(clear::Schema::RequestSchema(args)) =>
                    Ok(Request::ClearRequestSchema(clear::request_schema::Request::try_from(args)?)),
                Some(clear::Schema::ResponseSchema(args)) =>
                    Ok(Request::ClearResponseSchema(clear::response_schema::Request::try_from(args)?)),
            },
            Command::Functions { command } =>
                Ok(Request::Functions(functions::Request::try_from(command)?)),
            Command::Vector { command } =>
                Ok(Request::Vector(vector::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Agents(inner) => inner.into_command(),
            Request::Clear(inner) => inner.into_command(),
            Request::ClearRequestSchema(inner) => inner.into_command(),
            Request::ClearResponseSchema(inner) => inner.into_command(),
            Request::Functions(inner) => inner.into_command(),
            Request::Vector(inner) => inner.into_command(),
        }
    }
}
