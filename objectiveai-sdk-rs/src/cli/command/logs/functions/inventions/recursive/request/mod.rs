pub mod get;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Subscribe(subscribe::Command),
}
