//! `diverge run providers daemons …`.
//!
//! The requests over daemon records. One file per request, one folder
//! per family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over daemon records.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Daemons {
    /// Naming a daemon to connect to.
    Add(add::Arguments),
    /// Forgetting a daemon.
    Delete(delete::Arguments),
    /// Changing a daemon record.
    Edit(edit::Arguments),
    /// Getting one daemon.
    Get(get::Arguments),
    /// Listing a caller's daemons.
    List(list::Arguments),
    /// Putting tags on a daemon.
    Tag(tag::Arguments),
    /// Taking tags off a daemon.
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
        Daemons::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Daemons::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
