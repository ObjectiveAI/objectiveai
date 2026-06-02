pub mod agents;
pub mod clear;
pub mod functions;
pub mod vector;

#[derive(clap::Subcommand)]
pub enum Command {
    Agents {
        #[command(subcommand)]
        command: agents::Command,
    },
    Clear(clear::Command),
    Functions {
        #[command(subcommand)]
        command: functions::Command,
    },
    Vector {
        #[command(subcommand)]
        command: vector::Command,
    },
}
