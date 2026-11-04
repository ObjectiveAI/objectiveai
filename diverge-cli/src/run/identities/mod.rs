//! `diverge run identities …`.
//!
//! The requests over the daemon's host's identities: the issuers they
//! are signed in at, and the subjects at one issuer. Not `diverge
//! identities`, which signs this host's own in. One file per request:
//! the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod issuers;
pub mod subjects;

/// The requests over the daemon's host's identities.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Identities {
    /// Listing the issuers the daemon's host's identities are signed in
    /// at.
    Issuers(issuers::Arguments),
    /// Listing the subjects signed in at one issuer.
    Subjects(subjects::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Identities,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Identities::Issuers(arguments) => issuers::run(arguments, target, emitter).await,
        Identities::Subjects(arguments) => subjects::run(arguments, target, emitter).await,
    }
}
