//! `diverge run standalone-tools …`.
//!
//! The requests over standalone tools, and their templates. One file
//! per request, one folder per family within: the tree of files is the
//! tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod attach_agent_instance;
pub mod attach_agent_template;
pub mod connect;
pub mod create;
pub mod delete;
pub mod detach_agent_instance;
pub mod detach_agent_template;
pub mod download;
pub mod edit;
pub mod filetree;
pub mod get;
pub mod list;
pub mod logs;
pub mod register;
pub mod tag;
pub mod templates;
pub mod transfer;
pub mod untag;
pub mod upload;

/// The requests over standalone tools, and their templates.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum StandaloneTools {
    /// Attaching a tool to one agent, by their names.
    AttachAgentInstance(attach_agent_instance::Arguments),
    /// Attaching a tool to every agent of a template, by the tool's name
    /// and the template's id.
    AttachAgentTemplate(attach_agent_template::Arguments),
    /// Serving a tool the daemon runs to another daemon: the scope that
    /// daemon's agents reach the tool through, over the daemon
    /// connection, held for one MCP exchange.
    #[command(subcommand)]
    Connect(connect::Connect),
    /// Creating a tool under a name.
    Create(create::Arguments),
    /// Deleting a tool by name.
    Delete(delete::Arguments),
    /// Detaching a tool from one agent, by their names.
    DetachAgentInstance(detach_agent_instance::Arguments),
    /// Detaching a tool from every agent of a template, by the tool's
    /// name and the template's id.
    DetachAgentTemplate(detach_agent_template::Arguments),
    /// Downloading a file or a directory out of a tool.
    Download(download::Arguments),
    /// Editing a tool: its name, its mounts, its secrets.
    Edit(edit::Arguments),
    /// Watching a tool's container's filesystem, whole.
    Filetree(filetree::Arguments),
    /// Getting one tool.
    Get(get::Arguments),
    /// Listing a caller's tools.
    List(list::Arguments),
    /// Reading a tool's log, and watching it.
    Logs(logs::Arguments),
    /// Registering a tool another daemon holds, under a name.
    Register(register::Arguments),
    /// Putting tags on a tool.
    Tag(tag::Arguments),
    /// The requests over standalone tool templates.
    #[command(subcommand)]
    Templates(templates::Templates),
    /// Copying a file or a directory out of a tool's container into an
    /// agent, a tool, a dependency tool or a volume, without the client
    /// in between.
    Transfer(transfer::Arguments),
    /// Taking tags off a tool.
    Untag(untag::Arguments),
    /// Uploading files into a tool's container.
    Upload(upload::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: StandaloneTools,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        StandaloneTools::AttachAgentInstance(arguments) => {
            attach_agent_instance::run(arguments, target, emitter).await
        }
        StandaloneTools::AttachAgentTemplate(arguments) => {
            attach_agent_template::run(arguments, target, emitter).await
        }
        StandaloneTools::Connect(command) => connect::run(command, target, emitter).await,
        StandaloneTools::Create(arguments) => create::run(arguments, target, emitter).await,
        StandaloneTools::Delete(arguments) => delete::run(arguments, target, emitter).await,
        StandaloneTools::DetachAgentInstance(arguments) => {
            detach_agent_instance::run(arguments, target, emitter).await
        }
        StandaloneTools::DetachAgentTemplate(arguments) => {
            detach_agent_template::run(arguments, target, emitter).await
        }
        StandaloneTools::Download(arguments) => download::run(arguments, target, emitter).await,
        StandaloneTools::Edit(arguments) => edit::run(arguments, target, emitter).await,
        StandaloneTools::Filetree(arguments) => filetree::run(arguments, target, emitter).await,
        StandaloneTools::Get(arguments) => get::run(arguments, target, emitter).await,
        StandaloneTools::List(arguments) => list::run(arguments, target, emitter).await,
        StandaloneTools::Logs(arguments) => logs::run(arguments, target, emitter).await,
        StandaloneTools::Register(arguments) => register::run(arguments, target, emitter).await,
        StandaloneTools::Tag(arguments) => tag::run(arguments, target, emitter).await,
        StandaloneTools::Templates(command) => templates::run(command, target, emitter).await,
        StandaloneTools::Transfer(arguments) => transfer::run(arguments, target, emitter).await,
        StandaloneTools::Untag(arguments) => untag::run(arguments, target, emitter).await,
        StandaloneTools::Upload(arguments) => upload::run(arguments, target, emitter).await,
    }
}
