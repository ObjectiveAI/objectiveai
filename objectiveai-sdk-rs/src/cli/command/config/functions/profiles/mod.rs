pub mod favorites;
pub mod get;
pub mod pairs;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Favorites {
        #[command(subcommand)]
        command: favorites::Command,
    },
    Pairs {
        #[command(subcommand)]
        command: pairs::Command,
    },
}
