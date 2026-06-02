pub mod get;
pub mod set;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Set(set::Command),
}
