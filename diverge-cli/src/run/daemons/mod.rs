//! `diverge run daemons …`.
//!
//! The requests over the remote daemons' records: each one account of
//! ours on another daemon, keyed by an ID, which relays and connected
//! tools name it by.
//! One file per request: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod listen;
pub mod tag;
pub mod untag;

/// The requests over the remote daemons' records.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Daemons {
    /// Adding a remote daemon's record.
    Add(add::Arguments),
    /// Deleting a remote daemon's record.
    Delete(delete::Arguments),
    /// Changing a remote daemon's record.
    Edit(edit::Arguments),
    /// Getting one remote daemon's record.
    Get(get::Arguments),
    /// Listing the remote daemons' records.
    List(list::Arguments),
    /// Listening for the messages sent to the daemon.
    Listen(listen::Arguments),
    /// Putting tags on a remote daemon's record.
    Tag(tag::Arguments),
    /// Taking tags off a remote daemon's record.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Daemons,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Daemons::Add(arguments) => add::run(arguments, target, emitter).await,
        Daemons::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Daemons::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Daemons::Get(arguments) => get::run(arguments, target, emitter).await,
        Daemons::List(arguments) => list::run(arguments, target, emitter).await,
        Daemons::Listen(arguments) => listen::run(arguments, target, emitter).await,
        Daemons::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Daemons::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
