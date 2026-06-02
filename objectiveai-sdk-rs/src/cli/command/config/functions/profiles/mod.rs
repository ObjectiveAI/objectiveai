pub mod favorites;
pub mod get;
pub mod pairs;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Favorites {
        #[command(subcommand)]
        command: favorites::Command,
    },
    Pairs {
        #[command(subcommand)]
        command: pairs::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Favorites(favorites::Request),
    Pairs(pairs::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Favorites(favorites::ResponseItem),
    Pairs(pairs::ResponseItem),
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
            Command::Favorites { command } =>
                Ok(Request::Favorites(favorites::Request::try_from(command)?)),
            Command::Pairs { command } =>
                Ok(Request::Pairs(pairs::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Favorites(inner) => inner.into_command(),
            Request::Pairs(inner) => inner.into_command(),
        }
    }
}
