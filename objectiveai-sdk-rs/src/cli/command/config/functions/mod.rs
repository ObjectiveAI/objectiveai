pub mod favorites;
pub mod get;
pub mod inventions;
pub mod profiles;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Favorites {
        #[command(subcommand)]
        command: favorites::Command,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
    Profiles {
        #[command(subcommand)]
        command: profiles::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Favorites(favorites::Request),
    Inventions(inventions::Request),
    Profiles(profiles::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Favorites(favorites::ResponseItem),
    Inventions(inventions::Response),
    Profiles(profiles::ResponseItem),
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
            Command::Inventions { command } =>
                Ok(Request::Inventions(inventions::Request::try_from(command)?)),
            Command::Profiles { command } =>
                Ok(Request::Profiles(profiles::Request::try_from(command)?)),
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
            Request::Inventions(inner) => inner.into_command(),
            Request::Profiles(inner) => inner.into_command(),
        }
    }
}
