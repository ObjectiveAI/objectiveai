pub mod completions;

#[derive(clap::Subcommand)]
pub enum Command {
    Completions {
        #[command(subcommand)]
        command: completions::Command,
    },
}
