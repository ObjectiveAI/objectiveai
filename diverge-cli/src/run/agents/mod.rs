//! `diverge run agents …`.
//!
//! The requests over agents, and their templates. One file per request,
//! one folder per family within: the tree of files is the tree of
//! subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod create;
pub mod delete;
pub mod download;
pub mod edit;
pub mod filetree;
pub mod get;
pub mod list;
pub mod message;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod untag;
pub mod upload;

/// The requests over agents, and their templates.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Agents {
    /// Creating an agent under a name.
    Create(create::Arguments),
    /// Deleting an agent by name.
    Delete(delete::Arguments),
    /// Downloading a file or a directory out of an agent.
    Download(download::Arguments),
    /// Editing an agent: its name, its mounts.
    Edit(edit::Arguments),
    /// Watching an agent's container's filesystem, whole.
    Filetree(filetree::Arguments),
    /// Getting one agent.
    Get(get::Arguments),
    /// Listing a caller's agents.
    List(list::Arguments),
    /// Sending an agent a message.
    Message(message::Arguments),
    /// Putting tags on an agent.
    Tag(tag::Arguments),
    /// The requests over agent templates.
    #[command(subcommand)]
    Templates(templates::Templates),
    /// Copying a file or a directory out of an agent's container into
    /// an agent, a tool, a dependency tool or a volume, without the
    /// client in between.
    Transfer(transfer::Arguments),
    /// Taking tags off an agent.
    Untag(untag::Arguments),
    /// Uploading files into an agent's container.
    Upload(upload::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(command: Agents, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Agents::Create(arguments) => create::run(arguments, target, emitter).await,
        Agents::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Agents::Download(arguments) => download::run(arguments, target, emitter).await,
        Agents::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Agents::Filetree(arguments) => filetree::run(arguments, target, emitter).await,
        Agents::Get(arguments) => get::run(arguments, target, emitter).await,
        Agents::List(arguments) => list::run(arguments, target, emitter).await,
        Agents::Message(arguments) => message::run(arguments, target, emitter).await,
        Agents::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Agents::Templates(command) => templates::run(command, target, emitter).await,
        Agents::Transfer(arguments) => transfer::run(arguments, target, emitter).await,
        Agents::Untag(arguments) => untag::run(arguments, target, emitter).await,
        Agents::Upload(arguments) => upload::run(arguments, target, emitter).await,
    }
}
