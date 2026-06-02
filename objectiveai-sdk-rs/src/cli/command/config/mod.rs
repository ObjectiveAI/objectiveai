pub mod agents;
pub mod functions;
pub mod mcp;
pub mod swarms;
pub mod viewer;

#[derive(clap::Subcommand)]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: agents::Command,
    },
    Functions {
        #[command(subcommand)]
        command: functions::Command,
    },
    Mcp {
        #[command(subcommand)]
        command: mcp::Command,
    },
    Swarms {
        #[command(subcommand)]
        command: swarms::Command,
    },
    Viewer {
        #[command(subcommand)]
        command: viewer::Command,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Request {
    Agents(agents::Request),
    Functions(functions::Request),
    Mcp(mcp::Request),
    Swarms(swarms::Request),
    Viewer(viewer::Request),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseItem {
    Agents(agents::ResponseItem),
    Functions(functions::ResponseItem),
    Mcp(mcp::Response),
    Swarms(swarms::ResponseItem),
    Viewer(viewer::Response),
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Agents { command } =>
                Ok(Request::Agents(agents::Request::try_from(command)?)),
            Command::Functions { command } =>
                Ok(Request::Functions(functions::Request::try_from(command)?)),
            Command::Mcp { command } =>
                Ok(Request::Mcp(mcp::Request::try_from(command)?)),
            Command::Swarms { command } =>
                Ok(Request::Swarms(swarms::Request::try_from(command)?)),
            Command::Viewer { command } =>
                Ok(Request::Viewer(viewer::Request::try_from(command)?)),
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Agents(inner) => inner.into_command(),
            Request::Functions(inner) => inner.into_command(),
            Request::Mcp(inner) => inner.into_command(),
            Request::Swarms(inner) => inner.into_command(),
            Request::Viewer(inner) => inner.into_command(),
        }
    }
}
