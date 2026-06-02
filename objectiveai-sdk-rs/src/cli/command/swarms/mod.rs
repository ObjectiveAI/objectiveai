pub mod get;
pub mod list;
pub mod publish;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    List(list::Command),
    Publish(publish::Command),
}
