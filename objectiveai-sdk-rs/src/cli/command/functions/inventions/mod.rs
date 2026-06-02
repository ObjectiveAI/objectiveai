pub mod recursive;
pub mod state;

#[derive(clap::Subcommand)]
pub enum Command {
    Recursive {
        #[command(subcommand)]
        command: recursive::Command,
    },
    State {
        #[command(subcommand)]
        command: state::Command,
    },
}
