pub mod get;
pub mod messages;
pub mod notifications;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    Get(get::Command),
    Messages {
        #[command(subcommand)]
        command: messages::Command,
    },
    Notifications {
        #[command(subcommand)]
        command: notifications::Command,
    },
    Subscribe(subscribe::Command),
}
