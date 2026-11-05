//! `diverge run providers …`.
//!
//! The requests over providers — the ones the daemon's configuration
//! accepts. One file per
//! request, one folder per family within: the tree of files is the tree
//! of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod discover;
pub mod get;
pub mod list;

/// The requests over providers.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Providers {
    /// Getting one provider.
    Get(get::Arguments),
    /// Listing the providers.
    List(list::Arguments),
    /// Discovering peers through the connected providers: every
    /// identity discoverable on one, with the providers it was seen on.
    Discover(discover::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Providers,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Providers::Get(arguments) => get::run(arguments, target, emitter).await,
        Providers::List(arguments) => list::run(arguments, target, emitter).await,
        Providers::Discover(arguments) => discover::run(arguments, target, emitter).await,
    }
}
