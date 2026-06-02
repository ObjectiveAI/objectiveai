pub mod all;
pub mod id;
pub mod pending;
pub mod subscribe;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Read all items from each agent_instance_hierarchy.
    All(all::Command),
    /// Read a single item by its row id.
    Id(id::Command),
    /// Read pending items only.
    Pending(pending::Command),
    /// Subscribe to live updates for the given agents.
    Subscribe(subscribe::Command),
}
