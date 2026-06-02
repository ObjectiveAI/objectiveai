pub mod create;

#[derive(clap::Subcommand)]
pub enum Command {
    Create {
        #[command(subcommand)]
        command: create::Command,
    },
}
