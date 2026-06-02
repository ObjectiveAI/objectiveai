pub mod clear;
pub mod get;
pub mod list;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Clear(clear::Command),
    Get(get::Command),
    List(list::Command),
    Subscribe(subscribe::Command),
}
