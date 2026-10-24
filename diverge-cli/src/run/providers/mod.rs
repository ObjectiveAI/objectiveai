//! `diverge run providers …`.
//!
//! The requests over providers — the ones the daemon's configuration
//! accepts — and the remote daemons reached through them. One file per
//! request, one folder per family within: the tree of files is the tree
//! of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod daemons;
pub mod get;
pub mod list;

/// The requests over providers and the remote daemons reached through
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Providers {
    /// The requests over the remote daemons.
    #[command(subcommand)]
    Daemons(daemons::Daemons),
    /// Getting one provider.
    Get(get::Arguments),
    /// Listing the providers.
    List(list::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Providers,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Providers::Daemons(command) => daemons::run(command, target, emitter).await,
        Providers::Get(arguments) => get::run(arguments, target, emitter).await,
        Providers::List(arguments) => list::run(arguments, target, emitter).await,
    }
}
