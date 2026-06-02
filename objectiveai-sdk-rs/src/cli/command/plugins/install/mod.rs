pub mod filesystem;
pub mod github;

#[derive(clap::Subcommand)]
pub enum Command {
    Filesystem(filesystem::Command),
    Github(github::Command),
}
