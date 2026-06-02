pub mod get;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
}
