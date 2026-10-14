//! `diverge run providers outgoing …`.
//!
//! The requests over outgoing providers. One file per request, one
//! folder per family within: the tree of files is the tree of
//! subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod add;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over outgoing providers.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Outgoing {
    /// Naming a provider to dial.
    Add(add::Arguments),
    /// Forgetting an outgoing provider.
    Delete(delete::Arguments),
    /// Changing how an outgoing provider is dialled.
    Edit(edit::Arguments),
    /// Getting one outgoing provider.
    Get(get::Arguments),
    /// Listing a caller's outgoing providers.
    List(list::Arguments),
    /// Putting tags on an outgoing provider.
    Tag(tag::Arguments),
    /// Taking tags off an outgoing provider.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Outgoing,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Outgoing::Add(arguments) => add::run(arguments, target, emitter).await,
        Outgoing::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Outgoing::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Outgoing::Get(arguments) => get::run(arguments, target, emitter).await,
        Outgoing::List(arguments) => list::run(arguments, target, emitter).await,
        Outgoing::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Outgoing::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
