pub mod executions;
pub mod inventions;

#[derive(clap::Subcommand)]
pub enum Command {
    Executions {
        #[command(subcommand)]
        command: executions::Command,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
}
