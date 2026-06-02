pub mod get;
pub mod list;
pub mod me;
pub mod message;
pub mod publish;
pub mod read;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Get an agent by remote path or favorite name.
    Get(get::Command),
    /// List agents — `active` (direct children of the calling agent) or
    /// `available` (remote agents by source).
    List {
        #[command(subcommand)]
        command: list::Command,
    },
    /// Return the configured self agent id.
    Me(me::Command),
    /// Deliver a message to a running spawned agent (or resume its most
    /// recent completion via continuation if it's dormant).
    Message(message::Command),
    /// Publish an agent to the local filesystem.
    Publish(publish::Command),
    /// Read queue items.
    Read {
        #[command(subcommand)]
        command: read::Command,
    },
    /// Spawn an agent completion (open a streaming run as a child of this caller).
    Spawn(spawn::Command),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Get(get::Request),
    GetRequestSchema(get::request_schema::Request),
    GetResponseSchema(get::response_schema::Request),
    List(list::Request),
    Me(me::Request),
    MeRequestSchema(me::request_schema::Request),
    MeResponseSchema(me::response_schema::Request),
    Message(message::Request),
    MessageRequestSchema(message::request_schema::Request),
    MessageResponseSchema(message::response_schema::Request),
    Publish(publish::Request),
    PublishRequestSchema(publish::request_schema::Request),
    PublishResponseSchema(publish::response_schema::Request),
    Read(read::Request),
    Spawn(spawn::Request),
    SpawnRequestSchema(spawn::request_schema::Request),
    SpawnResponseSchema(spawn::response_schema::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Get(get::Response),
    GetRequestSchema(get::request_schema::Response),
    GetResponseSchema(get::response_schema::Response),
    List(list::ResponseItem),
    Me(me::Response),
    MeRequestSchema(me::request_schema::Response),
    MeResponseSchema(me::response_schema::Response),
    Message(message::Response),
    MessageRequestSchema(message::request_schema::Response),
    MessageResponseSchema(message::response_schema::Response),
    Publish(publish::Response),
    PublishRequestSchema(publish::request_schema::Response),
    PublishResponseSchema(publish::response_schema::Response),
    Read(read::ResponseItem),
    Spawn(spawn::ResponseItem),
    SpawnRequestSchema(spawn::request_schema::Response),
    SpawnResponseSchema(spawn::response_schema::Response),
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
            Command::List { command } =>
                Ok(Request::List(list::Request::try_from(command)?)),
            Command::Me(cmd) => match cmd.schema {
                None => Ok(Request::Me(me::Request::try_from(cmd.args)?)),
                Some(me::Schema::RequestSchema(args)) =>
                    Ok(Request::MeRequestSchema(me::request_schema::Request::try_from(args)?)),
                Some(me::Schema::ResponseSchema(args)) =>
                    Ok(Request::MeResponseSchema(me::response_schema::Request::try_from(args)?)),
            },
            Command::Message(cmd) => match cmd.schema {
                None => Ok(Request::Message(message::Request::try_from(cmd.args)?)),
                Some(message::Schema::RequestSchema(args)) =>
                    Ok(Request::MessageRequestSchema(message::request_schema::Request::try_from(args)?)),
                Some(message::Schema::ResponseSchema(args)) =>
                    Ok(Request::MessageResponseSchema(message::response_schema::Request::try_from(args)?)),
            },
            Command::Publish(cmd) => match cmd.schema {
                None => Ok(Request::Publish(publish::Request::try_from(cmd.args)?)),
                Some(publish::Schema::RequestSchema(args)) =>
                    Ok(Request::PublishRequestSchema(publish::request_schema::Request::try_from(args)?)),
                Some(publish::Schema::ResponseSchema(args)) =>
                    Ok(Request::PublishResponseSchema(publish::response_schema::Request::try_from(args)?)),
            },
            Command::Read { command } =>
                Ok(Request::Read(read::Request::try_from(command)?)),
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
            Request::Get(inner) => inner.into_command(),
            Request::GetRequestSchema(inner) => inner.into_command(),
            Request::GetResponseSchema(inner) => inner.into_command(),
            Request::List(inner) => inner.into_command(),
            Request::Me(inner) => inner.into_command(),
            Request::MeRequestSchema(inner) => inner.into_command(),
            Request::MeResponseSchema(inner) => inner.into_command(),
            Request::Message(inner) => inner.into_command(),
            Request::MessageRequestSchema(inner) => inner.into_command(),
            Request::MessageResponseSchema(inner) => inner.into_command(),
            Request::Publish(inner) => inner.into_command(),
            Request::PublishRequestSchema(inner) => inner.into_command(),
            Request::PublishResponseSchema(inner) => inner.into_command(),
            Request::Read(inner) => inner.into_command(),
            Request::Spawn(inner) => inner.into_command(),
            Request::SpawnRequestSchema(inner) => inner.into_command(),
            Request::SpawnResponseSchema(inner) => inner.into_command(),
        }
    }
}
