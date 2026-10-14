//! `diverge run volumes …`.
//!
//! The requests over volumes. One file per request, one folder per
//! family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod create;
pub mod delete;
pub mod download;
pub mod edit;
pub mod filetree;
pub mod get;
pub mod list;
pub mod stat;
pub mod tag;
pub mod transfer;
pub mod untag;
pub mod upload;

/// The requests over volumes.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Volumes {
    /// Creating a volume on a provider.
    Create(create::Arguments),
    /// Deleting a volume.
    Delete(delete::Arguments),
    /// Downloading a file or a directory out of a volume.
    Download(download::Arguments),
    /// Changing a volume: how many bytes it reserves, its mode, or
    /// both.
    Edit(edit::Arguments),
    /// Seeing what a volume holds: one snapshot of its tree.
    Filetree(filetree::Arguments),
    /// Getting one volume.
    Get(get::Arguments),
    /// Listing volumes, and keeping the list.
    List(list::Arguments),
    /// Walking a volume: how much of it is used, and the hash of its
    /// content.
    Stat(stat::Arguments),
    /// Putting tags on a volume.
    Tag(tag::Arguments),
    /// Copying a file or a directory out of a volume into an agent, a
    /// tool, a dependency tool or another volume, without the client in
    /// between.
    Transfer(transfer::Arguments),
    /// Taking tags off a volume.
    Untag(untag::Arguments),
    /// Uploading files into a volume.
    Upload(upload::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Volumes,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Volumes::Create(arguments) => create::run(arguments, target, emitter).await,
        Volumes::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Volumes::Download(arguments) => download::run(arguments, target, emitter).await,
        Volumes::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Volumes::Filetree(arguments) => filetree::run(arguments, target, emitter).await,
        Volumes::Get(arguments) => get::run(arguments, target, emitter).await,
        Volumes::List(arguments) => list::run(arguments, target, emitter).await,
        Volumes::Stat(arguments) => stat::run(arguments, target, emitter).await,
        Volumes::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Volumes::Transfer(arguments) => transfer::run(arguments, target, emitter).await,
        Volumes::Untag(arguments) => untag::run(arguments, target, emitter).await,
        Volumes::Upload(arguments) => upload::run(arguments, target, emitter).await,
    }
}
