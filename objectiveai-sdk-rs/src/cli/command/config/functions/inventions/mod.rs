pub mod get;
pub mod remote;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Remote {
        #[command(subcommand)]
        command: remote::Command,
    },
}
