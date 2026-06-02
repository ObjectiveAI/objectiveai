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
