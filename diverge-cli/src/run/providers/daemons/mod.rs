//! `diverge run providers daemons …`.
//!
//! The requests over the remote daemons of the daemon's configuration.
//! One file per request, one folder per family within: the tree of
//! files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod get;
pub mod list;

/// The requests over the remote daemons.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Daemons {
    /// Getting one remote daemon.
    Get(get::Arguments),
    /// Listing the remote daemons.
    List(list::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Daemons,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Daemons::Get(arguments) => get::run(arguments, target, emitter).await,
        Daemons::List(arguments) => list::run(arguments, target, emitter).await,
    }
}
