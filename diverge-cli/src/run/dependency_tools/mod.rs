//! `diverge run dependency-tools …`.
//!
//! The requests over dependency tools. One file per request, one folder
//! per family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod delete;
pub mod download;
pub mod exec;
pub mod filetree;
pub mod get;
pub mod list;
pub mod tag;
pub mod terminal;
pub mod transfer;
pub mod untag;
pub mod upload;

/// The requests over dependency tools.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DependencyTools {
    /// Deleting a dependency template's tools, by the template's id.
    Delete(delete::Arguments),
    /// Downloading a file or a directory out of a dependency tool.
    Download(download::Arguments),
    /// Watching a dependency tool's container's filesystem, whole.
    Filetree(filetree::Arguments),
    /// Running one command in a dependency tool's container, its output streamed.
    Exec(exec::Arguments),
    /// An interactive shell in a dependency tool's container, on a terminal.
    Terminal(terminal::Arguments),
    /// Getting one dependency tool.
    Get(get::Arguments),
    /// Listing a caller's dependency tools.
    List(list::Arguments),
    /// Putting tags on a dependency tool.
    Tag(tag::Arguments),
    /// Copying a file or a directory out of a dependency tool's
    /// container into an agent, a tool, a dependency tool or a volume,
    /// without the client in between.
    Transfer(transfer::Arguments),
    /// Taking tags off a dependency tool.
    Untag(untag::Arguments),
    /// Uploading files into a dependency tool's container.
    Upload(upload::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: DependencyTools,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        DependencyTools::Delete(arguments) => delete::run(arguments, target, emitter).await,
        DependencyTools::Download(arguments) => download::run(arguments, target, emitter).await,
        DependencyTools::Exec(arguments) => exec::run(arguments, target, emitter).await,
        DependencyTools::Terminal(arguments) => terminal::run(arguments, target, emitter).await,
        DependencyTools::Filetree(arguments) => filetree::run(arguments, target, emitter).await,
        DependencyTools::Get(arguments) => get::run(arguments, target, emitter).await,
        DependencyTools::List(arguments) => list::run(arguments, target, emitter).await,
        DependencyTools::Tag(arguments) => tag::run(arguments, target, emitter).await,
        DependencyTools::Transfer(arguments) => transfer::run(arguments, target, emitter).await,
        DependencyTools::Untag(arguments) => untag::run(arguments, target, emitter).await,
        DependencyTools::Upload(arguments) => upload::run(arguments, target, emitter).await,
    }
}
