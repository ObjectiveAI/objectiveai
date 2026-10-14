//! `diverge run postgres …`.
//!
//! The requests over the database. One file per request, one folder per
//! family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod get;
pub mod list;

/// The requests over the database.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Postgres {
    /// Reading which database the daemon serves.
    Get(get::Arguments),
    /// Listing the container connections open through the database, and
    /// keeping the list.
    List(list::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Postgres,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Postgres::Get(arguments) => get::run(arguments, target, emitter).await,
        Postgres::List(arguments) => list::run(arguments, target, emitter).await,
    }
}
