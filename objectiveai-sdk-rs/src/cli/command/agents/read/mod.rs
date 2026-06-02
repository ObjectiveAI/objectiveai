pub mod all;
pub mod id;
pub mod pending;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read all items from each agent_instance_hierarchy.
    All(all::Args),
    /// Read a single item by its row id.
    Id(id::Args),
    /// Read pending items only.
    Pending(pending::Args),
    /// Subscribe to live updates for the given agents.
    Subscribe(subscribe::Args),
}
