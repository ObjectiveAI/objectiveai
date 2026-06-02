pub mod address;
pub mod get;
pub mod port;
pub mod secret;
pub mod signature;

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
    Secret {
        #[command(subcommand)]
        command: secret::Command,
    },
    Signature {
        #[command(subcommand)]
        command: signature::Command,
    },
}
