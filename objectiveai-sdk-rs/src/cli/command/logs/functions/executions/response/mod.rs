pub mod clear;
pub mod get;
pub mod list;
pub mod retry_tokens;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Clear(clear::Command),
    Get(get::Command),
    List(list::Command),
    RetryTokens {
        #[command(subcommand)]
        command: retry_tokens::Command,
    },
    Subscribe(subscribe::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Clear(clear::Request),
    ClearRequestSchema(clear::request_schema::Request),
    ClearResponseSchema(clear::response_schema::Request),
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    List(list::Request),
    ListRequestSchema(list::request_schema::Request),
    ListResponseSchema(list::response_schema::Request),
    RetryTokens(retry_tokens::Request),
    Subscribe(subscribe::Request),
    SubscribeRequestSchema(subscribe::request_schema::Request),
    SubscribeResponseSchema(subscribe::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Clear(clear::Response),
    ClearRequestSchema(clear::request_schema::Response),
    ClearResponseSchema(clear::response_schema::Response),
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    List(list::ResponseItem),
    ListRequestSchema(list::request_schema::Response),
    ListResponseSchema(list::response_schema::Response),
    RetryTokens(retry_tokens::Response),
    Subscribe(subscribe::Response),
    SubscribeRequestSchema(subscribe::request_schema::Response),
    SubscribeResponseSchema(subscribe::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Clear(cmd) => match cmd.schema {
                None => Ok(Request::Clear(clear::Request::try_from(cmd.args)?)),
                Some(clear::Schema::RequestSchema(args)) =>
                    Ok(Request::ClearRequestSchema(clear::request_schema::Request::try_from(args)?)),
                Some(clear::Schema::ResponseSchema(args)) =>
                    Ok(Request::ClearResponseSchema(clear::response_schema::Request::try_from(args)?)),
            },
            Command::Get(cmd) => match cmd.schema {
                None => Ok(Request::Get(get::Request::try_from(cmd.args)?)),
                Some(get::Schema::RequestSchema(args)) =>
                    Ok(Request::GetRequestSchema(get::request_schema::Request::try_from(args)?)),
                Some(get::Schema::ResponseSchema(args)) =>
                    Ok(Request::GetResponseSchema(get::response_schema::Request::try_from(args)?)),
            },
            Command::List(cmd) => match cmd.schema {
                None => Ok(Request::List(list::Request::try_from(cmd.args)?)),
                Some(list::Schema::RequestSchema(args)) =>
                    Ok(Request::ListRequestSchema(list::request_schema::Request::try_from(args)?)),
                Some(list::Schema::ResponseSchema(args)) =>
                    Ok(Request::ListResponseSchema(list::response_schema::Request::try_from(args)?)),
            },
            Command::RetryTokens { command } =>
                Ok(Request::RetryTokens(retry_tokens::Request::try_from(command)?)),
            Command::Subscribe(cmd) => match cmd.schema {
                None => Ok(Request::Subscribe(subscribe::Request::try_from(cmd.args)?)),
                Some(subscribe::Schema::RequestSchema(args)) =>
                    Ok(Request::SubscribeRequestSchema(subscribe::request_schema::Request::try_from(args)?)),
                Some(subscribe::Schema::ResponseSchema(args)) =>
                    Ok(Request::SubscribeResponseSchema(subscribe::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Clear(inner) => inner.into_command(),
            Request::ClearRequestSchema(inner) => inner.into_command(),
            Request::ClearResponseSchema(inner) => inner.into_command(),
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::ListRequestSchema(inner) => inner.into_command(),
            Request::ListResponseSchema(inner) => inner.into_command(),
            Request::RetryTokens(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
        }
    }
}
