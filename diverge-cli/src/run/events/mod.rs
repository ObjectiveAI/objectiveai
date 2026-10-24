//! `diverge run events …`.
//!
//! The requests over events: the graph of who sent what to whom
//! through the daemon. One file per request: the tree of files is the
//! tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod list;

/// The requests over events.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Events {
    /// Listing the events, narrowed, and keeping the list.
    List(list::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(command: Events, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Events::List(arguments) => list::run(arguments, target, emitter).await,
    }
}
