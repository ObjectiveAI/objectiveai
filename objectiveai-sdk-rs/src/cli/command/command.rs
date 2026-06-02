//! Root-level clap `Command` plus the matching typed `Request` /
//! `ResponseItem` aggregators and conversions, mirroring the same
//! pattern every tier `mod.rs` follows for its own subcommands.

#[derive(clap::Subcommand)]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: super::agents::Command,
    },
    Config {
        #[command(subcommand)]
        command: super::config::Command,
    },
    Functions {
        #[command(subcommand)]
        command: super::functions::Command,
    },
    Logs {
        #[command(subcommand)]
        command: super::logs::Command,
    },
    Mcp {
        #[command(subcommand)]
        command: super::mcp::Command,
    },
    Plugins {
        #[command(subcommand)]
        command: super::plugins::Command,
    },
    Swarms {
        #[command(subcommand)]
        command: super::swarms::Command,
    },
    Tools {
        #[command(subcommand)]
        command: super::tools::Command,
    },
    Update(super::update::Command),
    Viewer {
        #[command(subcommand)]
        command: super::viewer::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Agents(super::agents::Request),
    Config(super::config::Request),
    Functions(super::functions::Request),
    Logs(super::logs::Request),
    Mcp(super::mcp::Request),
    Plugins(super::plugins::Request),
    Swarms(super::swarms::Request),
    Tools(super::tools::Request),
    Update(super::update::Request),
    UpdateRequestSchema(super::update::request_schema::Request),
    UpdateResponseSchema(super::update::response_schema::Request),
    Viewer(super::viewer::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Agents(super::agents::ResponseItem),
    Config(super::config::ResponseItem),
    Functions(super::functions::ResponseItem),
    Logs(super::logs::ResponseItem),
    Mcp(super::mcp::Response),
    Plugins(super::plugins::ResponseItem),
    Swarms(super::swarms::ResponseItem),
    Tools(super::tools::ResponseItem),
    Update(super::update::Response),
    UpdateRequestSchema(super::update::request_schema::Response),
    UpdateResponseSchema(super::update::response_schema::Response),
    Viewer(super::viewer::Response),
}

impl TryFrom<Command> for Request {
    type Error = super::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Agents { command } =>
                Ok(Request::Agents(super::agents::Request::try_from(command)?)),
            Command::Config { command } =>
                Ok(Request::Config(super::config::Request::try_from(command)?)),
            Command::Functions { command } =>
                Ok(Request::Functions(super::functions::Request::try_from(command)?)),
            Command::Logs { command } =>
                Ok(Request::Logs(super::logs::Request::try_from(command)?)),
            Command::Mcp { command } =>
                Ok(Request::Mcp(super::mcp::Request::try_from(command)?)),
            Command::Plugins { command } =>
                Ok(Request::Plugins(super::plugins::Request::try_from(command)?)),
            Command::Swarms { command } =>
                Ok(Request::Swarms(super::swarms::Request::try_from(command)?)),
            Command::Tools { command } =>
                Ok(Request::Tools(super::tools::Request::try_from(command)?)),
            Command::Update(cmd) => match cmd.schema {
                None => Ok(Request::Update(super::update::Request::try_from(cmd.args)?)),
                Some(super::update::Schema::RequestSchema(args)) =>
                    Ok(Request::UpdateRequestSchema(super::update::request_schema::Request::try_from(args)?)),
                Some(super::update::Schema::ResponseSchema(args)) =>
                    Ok(Request::UpdateResponseSchema(super::update::response_schema::Request::try_from(args)?)),
            },
            Command::Viewer { command } =>
                Ok(Request::Viewer(super::viewer::Request::try_from(command)?)),
        }
    }
}

impl super::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Agents(inner) => inner.into_command(),
            Request::Config(inner) => inner.into_command(),
            Request::Functions(inner) => inner.into_command(),
            Request::Logs(inner) => inner.into_command(),
            Request::Mcp(inner) => inner.into_command(),
            Request::Plugins(inner) => inner.into_command(),
            Request::Swarms(inner) => inner.into_command(),
            Request::Tools(inner) => inner.into_command(),
            Request::Update(inner) => inner.into_command(),
            Request::UpdateRequestSchema(inner) => inner.into_command(),
            Request::UpdateResponseSchema(inner) => inner.into_command(),
            Request::Viewer(inner) => inner.into_command(),
        }
    }
}
