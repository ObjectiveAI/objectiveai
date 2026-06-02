pub mod favorites;
pub mod get;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Favorites {
        #[command(subcommand)]
        command: favorites::Command,
    },
}
