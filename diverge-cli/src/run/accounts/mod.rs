//! `diverge run accounts …`.
//!
//! The requests over accounts. One file per request, one folder per
//! family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over accounts.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Accounts {
    /// Creating an account.
    Create(create::Arguments),
    /// Deleting an account.
    Delete(delete::Arguments),
    /// Changing an account.
    Edit(edit::Arguments),
    /// Getting one account.
    Get(get::Arguments),
    /// Listing accounts.
    List(list::Arguments),
    /// Putting tags on an account.
    Tag(tag::Arguments),
    /// Taking tags off an account.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Accounts,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Accounts::Create(arguments) => create::run(arguments, target, emitter).await,
        Accounts::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Accounts::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Accounts::Get(arguments) => get::run(arguments, target, emitter).await,
        Accounts::List(arguments) => list::run(arguments, target, emitter).await,
        Accounts::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Accounts::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
