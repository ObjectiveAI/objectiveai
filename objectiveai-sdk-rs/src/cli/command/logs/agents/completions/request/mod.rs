pub mod get;
pub mod messages;
pub mod notifications;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Messages {
        #[command(subcommand)]
        command: messages::Command,
    },
    Notifications {
        #[command(subcommand)]
        command: notifications::Command,
    },
    Subscribe(subscribe::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    Messages(messages::Request),
    Notifications(notifications::Request),
    Subscribe(subscribe::Request),
    SubscribeRequestSchema(subscribe::request_schema::Request),
    SubscribeResponseSchema(subscribe::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    Messages(messages::Response),
    Notifications(notifications::Response),
    Subscribe(subscribe::Response),
    SubscribeRequestSchema(subscribe::request_schema::Response),
    SubscribeResponseSchema(subscribe::response_schema::Response),
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
            Command::Messages { command } =>
                Ok(Request::Messages(messages::Request::try_from(command)?)),
            Command::Notifications { command } =>
                Ok(Request::Notifications(notifications::Request::try_from(command)?)),
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
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::Messages(inner) => inner.into_command(),
            Request::Notifications(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
        }
    }
}
