pub mod active;
pub mod available;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Direct children of the calling agent.
    Active(active::Command),
    /// Remote agents available from a given source.
    Available(available::Command),
}
