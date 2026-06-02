pub mod clear;
pub mod get;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Clear(clear::Command),
    Get(get::Command),
    Subscribe(subscribe::Command),
}
