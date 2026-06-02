pub mod favorites;
pub mod get;
pub mod inventions;
pub mod profiles;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Favorites {
        #[command(subcommand)]
        command: favorites::Command,
    },
    Inventions {
        #[command(subcommand)]
        command: inventions::Command,
    },
    Profiles {
        #[command(subcommand)]
        command: profiles::Command,
    },
}
