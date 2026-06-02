pub mod address;
pub mod get;
pub mod port;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Address {
        #[command(subcommand)]
        command: address::Command,
    },
    Port {
        #[command(subcommand)]
        command: port::Command,
    },
}
