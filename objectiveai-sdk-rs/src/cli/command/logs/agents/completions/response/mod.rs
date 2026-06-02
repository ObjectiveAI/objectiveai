pub mod clear;
pub mod continuations;
pub mod get;
pub mod list;
pub mod messages;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Clear(clear::Command),
    Continuations {
        #[command(subcommand)]
        command: continuations::Command,
    },
    Get(get::Command),
    List(list::Command),
    Messages {
        #[command(subcommand)]
        command: messages::Command,
    },
    Subscribe(subscribe::Command),
}
