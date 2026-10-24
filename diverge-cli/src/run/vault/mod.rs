//! `diverge run vault …`.
//!
//! The requests over the vault. One file per request, one folder per
//! family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod delete;
pub mod get;
pub mod insert;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over the vault.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Vault {
    /// Deleting a vault key.
    Delete(delete::Arguments),
    /// Getting one vault key, with its slots.
    Get(get::Arguments),
    /// Inserting a value into one vault key.
    Insert(insert::Arguments),
    /// Listing vault keys.
    List(list::Arguments),
    /// Putting tags on a vault key.
    Tag(tag::Arguments),
    /// Taking tags off a vault key.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(command: Vault, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Vault::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Vault::Get(arguments) => get::run(arguments, target, emitter).await,
        Vault::Insert(arguments) => insert::run(arguments, target, emitter).await,
        Vault::List(arguments) => list::run(arguments, target, emitter).await,
        Vault::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Vault::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
