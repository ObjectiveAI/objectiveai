pub mod executions;
pub mod get;
pub mod inventions;
pub mod list;
pub mod profiles;
pub mod publish;

#[derive(clap::Subcommand)]
pub enum Command {
    Executions {
        #[command(subcommand)]
        command: executions::Command,
    },
    Get(get::Command),
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
    List(list::Command),
    Profiles {
        #[command(subcommand)]
        command: profiles::Command,
    },
    Publish(publish::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Executions(executions::Request),
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Inventions(inventions::Request),
    List(list::Request),
    ListRequestSchema(list::request_schema::Request),
    ListResponseSchema(list::response_schema::Request),
    Profiles(profiles::Request),
    Publish(publish::Request),
    PublishRequestSchema(publish::request_schema::Request),
    PublishResponseSchema(publish::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Executions(executions::ResponseItem),
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Inventions(inventions::ResponseItem),
    List(list::ResponseItem),
    ListRequestSchema(list::request_schema::Response),
    ListResponseSchema(list::response_schema::Response),
    Profiles(profiles::ResponseItem),
    Publish(publish::Response),
    PublishRequestSchema(publish::request_schema::Response),
    PublishResponseSchema(publish::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Executions { command } =>
                Ok(Request::Executions(executions::Request::try_from(command)?)),
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::Inventions { command } =>
                Ok(Request::Inventions(inventions::Request::try_from(command)?)),
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::Profiles { command } =>
                Ok(Request::Profiles(profiles::Request::try_from(command)?)),
            Command::Publish(cmd) => match cmd.schema {
                None => Ok(Request::Publish(publish::Request::try_from(cmd.args)?)),
                Some(publish::Schema::RequestSchema(args)) =>
                    Ok(Request::PublishRequestSchema(publish::request_schema::Request::try_from(args)?)),
                Some(publish::Schema::ResponseSchema(args)) =>
                    Ok(Request::PublishResponseSchema(publish::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Executions(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Inventions(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::Profiles(inner) => inner.into_command(),
            Request::Publish(inner) => inner.into_command(),
            Request::PublishRequestSchema(inner) => inner.into_command(),
            Request::PublishResponseSchema(inner) => inner.into_command(),
        }
    }
}
