pub mod all;
pub mod id;
pub mod pending;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read all items from each agent_instance_hierarchy.
    All(all::Command),
    /// Read a single item by its row id.
    Id(id::Command),
    /// Read pending items only.
    Pending(pending::Command),
    /// Subscribe to live updates for the given agents.
    Subscribe(subscribe::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    All(all::Request),
    AllRequestSchema(all::request_schema::Request),
    AllResponseSchema(all::response_schema::Request),
    Id(id::Request),
    IdRequestSchema(id::request_schema::Request),
    IdResponseSchema(id::response_schema::Request),
    Pending(pending::Request),
    PendingRequestSchema(pending::request_schema::Request),
    PendingResponseSchema(pending::response_schema::Request),
    Subscribe(subscribe::Request),
    SubscribeRequestSchema(subscribe::request_schema::Request),
    SubscribeResponseSchema(subscribe::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    All(all::ResponseItem),
    AllRequestSchema(all::request_schema::Response),
    AllResponseSchema(all::response_schema::Response),
    Id(id::Response),
    IdRequestSchema(id::request_schema::Response),
    IdResponseSchema(id::response_schema::Response),
    Pending(pending::ResponseItem),
    PendingRequestSchema(pending::request_schema::Response),
    PendingResponseSchema(pending::response_schema::Response),
    Subscribe(subscribe::ResponseItem),
    SubscribeRequestSchema(subscribe::request_schema::Response),
    SubscribeResponseSchema(subscribe::response_schema::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::All(cmd) => match cmd.schema {
                None => Ok(Request::All(all::Request::try_from(cmd.args)?)),
                Some(all::Schema::RequestSchema(args)) =>
                    Ok(Request::AllRequestSchema(all::request_schema::Request::try_from(args)?)),
                Some(all::Schema::ResponseSchema(args)) =>
                    Ok(Request::AllResponseSchema(all::response_schema::Request::try_from(args)?)),
            },
            Command::Id(cmd) => match cmd.schema {
                None => Ok(Request::Id(id::Request::try_from(cmd.args)?)),
                Some(id::Schema::RequestSchema(args)) =>
                    Ok(Request::IdRequestSchema(id::request_schema::Request::try_from(args)?)),
                Some(id::Schema::ResponseSchema(args)) =>
                    Ok(Request::IdResponseSchema(id::response_schema::Request::try_from(args)?)),
            },
            Command::Pending(cmd) => match cmd.schema {
                None => Ok(Request::Pending(pending::Request::try_from(cmd.args)?)),
                Some(pending::Schema::RequestSchema(args)) =>
                    Ok(Request::PendingRequestSchema(pending::request_schema::Request::try_from(args)?)),
                Some(pending::Schema::ResponseSchema(args)) =>
                    Ok(Request::PendingResponseSchema(pending::response_schema::Request::try_from(args)?)),
            },
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
            Request::All(inner) => inner.into_command(),
            Request::AllRequestSchema(inner) => inner.into_command(),
            Request::AllResponseSchema(inner) => inner.into_command(),
            Request::Id(inner) => inner.into_command(),
            Request::IdRequestSchema(inner) => inner.into_command(),
            Request::IdResponseSchema(inner) => inner.into_command(),
            Request::Pending(inner) => inner.into_command(),
            Request::PendingRequestSchema(inner) => inner.into_command(),
            Request::PendingResponseSchema(inner) => inner.into_command(),
            Request::Subscribe(inner) => inner.into_command(),
            Request::SubscribeRequestSchema(inner) => inner.into_command(),
            Request::SubscribeResponseSchema(inner) => inner.into_command(),
        }
    }
}
