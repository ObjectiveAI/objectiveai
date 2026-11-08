//! `diverge run providers daemons …`.
//!
//! The requests over the daemons reached through the providers: who is
//! discoverable on them, and the direct messages sent through them.
//! One file per request: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod discover;
pub mod listen;
pub mod message;

/// The requests over the daemons reached through the providers.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Daemons {
    /// Discovering peers through the connected providers: every
    /// identity discoverable on one, with the providers it was seen on.
    Discover(discover::Arguments),
    /// Listening for the messages sent to the daemon.
    Listen(listen::Arguments),
    /// Sending one direct message through the daemon's providers.
    Message(message::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Daemons,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Daemons::Discover(arguments) => discover::run(arguments, target, emitter).await,
        Daemons::Listen(arguments) => listen::run(arguments, target, emitter).await,
        Daemons::Message(arguments) => message::run(arguments, target, emitter).await,
    }
}
