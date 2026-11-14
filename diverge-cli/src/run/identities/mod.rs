//! `diverge run identities …`.
//!
//! The requests over the daemon's host's identities: listed, each with
//! its tags; one of them got; their tags put on and taken off; and one signed in there. Not `diverge identities`, which works on this host's own. One file per request:
//! the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod get;
pub mod list;
pub mod login;
pub mod tag;
pub mod untag;

/// The requests over the daemon's host's identities.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Identities {
    /// Getting one of the daemon's host's identities, with its tags.
    Get(get::Arguments),
    /// Listing the daemon's host's identities, each with its tags.
    List(list::Arguments),
    /// Putting tags on an identity.
    Tag(tag::Arguments),
    /// Taking tags off an identity.
    Untag(untag::Arguments),
    /// Signing an identity in on the daemon's host: the code printed,
    /// and approved in a browser here or anywhere.
    Login(login::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Identities,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Identities::Get(arguments) => get::run(arguments, target, emitter).await,
        Identities::List(arguments) => list::run(arguments, target, emitter).await,
        Identities::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Identities::Untag(arguments) => untag::run(arguments, target, emitter).await,
        Identities::Login(arguments) => login::run(arguments, target, emitter).await,
    }
}
