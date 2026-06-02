pub mod standard;
pub mod swiss_system;

#[derive(clap::Subcommand)]
pub enum Command {
    Standard(standard::Command),
    SwissSystem(swiss_system::Command),
}
