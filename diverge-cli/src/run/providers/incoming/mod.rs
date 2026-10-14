//! `diverge run providers incoming …`.
//!
//! The requests over incoming credentials. One file per request, one
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

/// The requests over incoming credentials.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Incoming {
    /// Adding a credential of incoming providers.
    Add(add::Arguments),
    /// Taking a credential out.
    Delete(delete::Arguments),
    /// Replacing a credential.
    Edit(edit::Arguments),
    /// Getting one credential.
    Get(get::Arguments),
    /// Listing the credentials of incoming providers.
    List(list::Arguments),
    /// Putting tags on an incoming credential.
    Tag(tag::Arguments),
    /// Taking tags off an incoming credential.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Incoming,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Incoming::Add(arguments) => add::run(arguments, target, emitter).await,
        Incoming::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Incoming::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Incoming::Get(arguments) => get::run(arguments, target, emitter).await,
        Incoming::List(arguments) => list::run(arguments, target, emitter).await,
        Incoming::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Incoming::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
