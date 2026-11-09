//! `diverge run connected-standalone-tools …`.
//!
//! The requests over connected standalone tools: another daemon's
//! standalone tools, held here under names, for this daemon's agents.
//! One file per request: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod attach_agent_instance;
pub mod attach_agent_template;
pub mod create;
pub mod delete;
pub mod detach_agent_instance;
pub mod detach_agent_template;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over connected standalone tools.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum ConnectedStandaloneTools {
    /// Attaching a connected tool to one agent, by their names.
    AttachAgentInstance(attach_agent_instance::Arguments),
    /// Attaching a connected tool to every agent of a template, by the
    /// tool's name and the template's id.
    AttachAgentTemplate(attach_agent_template::Arguments),
    /// Holding a standalone tool another daemon holds, under a name.
    Create(create::Arguments),
    /// Deleting a connected tool by name.
    Delete(delete::Arguments),
    /// Detaching a connected tool from one agent, by their names.
    DetachAgentInstance(detach_agent_instance::Arguments),
    /// Detaching a connected tool from every agent of a template, by the
    /// tool's name and the template's id.
    DetachAgentTemplate(detach_agent_template::Arguments),
    /// Editing a connected tool: its name.
    Edit(edit::Arguments),
    /// Getting one connected tool.
    Get(get::Arguments),
    /// Listing a caller's connected tools.
    List(list::Arguments),
    /// Putting tags on a connected tool.
    Tag(tag::Arguments),
    /// Taking tags off a connected tool.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: ConnectedStandaloneTools,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        ConnectedStandaloneTools::AttachAgentInstance(arguments) => {
            attach_agent_instance::run(arguments, target, emitter).await
        }
        ConnectedStandaloneTools::AttachAgentTemplate(arguments) => {
            attach_agent_template::run(arguments, target, emitter).await
        }
        ConnectedStandaloneTools::Create(arguments) => {
            create::run(arguments, target, emitter).await
        }
        ConnectedStandaloneTools::Delete(arguments) => {
            delete::run(arguments, target, emitter).await
        }
        ConnectedStandaloneTools::DetachAgentInstance(arguments) => {
            detach_agent_instance::run(arguments, target, emitter).await
        }
        ConnectedStandaloneTools::DetachAgentTemplate(arguments) => {
            detach_agent_template::run(arguments, target, emitter).await
        }
        ConnectedStandaloneTools::Edit(arguments) => edit::run(arguments, target, emitter).await,
        ConnectedStandaloneTools::Get(arguments) => get::run(arguments, target, emitter).await,
        ConnectedStandaloneTools::List(arguments) => list::run(arguments, target, emitter).await,
        ConnectedStandaloneTools::Tag(arguments) => tag::run(arguments, target, emitter).await,
        ConnectedStandaloneTools::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
