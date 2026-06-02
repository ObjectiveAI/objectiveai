pub mod executions;
pub mod get;
pub mod inventions;
pub mod list;
pub mod profiles;
pub mod publish;

#[derive(clap::Subcommand)]
pub enum Command {
    Executions {
        #[command(subcommand)]
        command: executions::Command,
    },
    Get(get::Command),
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
    List(list::Command),
    Profiles {
        #[command(subcommand)]
        command: profiles::Command,
    },
    Publish(publish::Command),
}
