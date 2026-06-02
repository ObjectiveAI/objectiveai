pub mod get;
pub mod list;
pub mod me;
pub mod message;
pub mod publish;
pub mod read;
pub mod spawn;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Get an agent by remote path or favorite name.
    Get(get::Command),
    /// List agents — `active` (direct children of the calling agent) or
    /// `available` (remote agents by source).
    List {
        #[command(subcommand)]
        command: list::Command,
    },
    /// Return the configured self agent id.
    Me(me::Command),
    /// Deliver a message to a running spawned agent (or resume its most
    /// recent completion via continuation if it's dormant).
    Message(message::Command),
    /// Publish an agent to the local filesystem.
    Publish(publish::Command),
    /// Read queue items.
    Read {
        #[command(subcommand)]
        command: read::Command,
    },
    /// Spawn an agent completion (open a streaming run as a child of this caller).
    Spawn(spawn::Command),
}
