//! `diverge run providers …`.
//!
//! The requests over providers: those dialled, those that dial in, and
//! daemons reached through them. One file per request, one folder per
//! family within: the tree of files is the tree of subcommands.

use clap::Subcommand;
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error};

pub mod daemons;
pub mod incoming;
pub mod outgoing;

/// The requests over providers: those dialled, those that dial in, and
/// daemons reached through them.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Providers {
    /// The requests over daemon records.
    #[command(subcommand)]
    Daemons(daemons::Daemons),
    /// The requests over incoming credentials.
    #[command(subcommand)]
    Incoming(incoming::Incoming),
    /// The requests over outgoing providers.
    #[command(subcommand)]
    Outgoing(outgoing::Outgoing),
}

/// The request put, by its subcommand.
pub async fn run(command: Providers, handle: &Handle, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Providers::Daemons(command) => daemons::run(command, handle, emitter).await,
        Providers::Incoming(command) => incoming::run(command, handle, emitter).await,
        Providers::Outgoing(command) => outgoing::run(command, handle, emitter).await,
    }
}
