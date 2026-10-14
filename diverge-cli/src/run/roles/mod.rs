//! `diverge run roles …`.
//!
//! The requests over roles. One file per request, one folder per family
//! within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod create;
pub mod delete;
pub mod edit;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over roles.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Roles {
    /// Creating a role.
    Create(create::Arguments),
    /// Deleting a role.
    Delete(delete::Arguments),
    /// Changing a role.
    Edit(edit::Arguments),
    /// Getting one role.
    Get(get::Arguments),
    /// Listing roles.
    List(list::Arguments),
    /// Putting tags on a role.
    Tag(tag::Arguments),
    /// Taking tags off a role.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(command: Roles, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Roles::Create(arguments) => create::run(arguments, target, emitter).await,
        Roles::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Roles::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Roles::Get(arguments) => get::run(arguments, target, emitter).await,
        Roles::List(arguments) => list::run(arguments, target, emitter).await,
        Roles::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Roles::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
