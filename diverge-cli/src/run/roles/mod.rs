//! `diverge run roles …`.
//!
//! The requests over roles. One file per request, one folder per family
//! within: the tree of files is the tree of subcommands.

use clap::Subcommand;
use diverge_sdk::wire::client::handle::Handle;

use crate::run::{Emitter, Error};

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
pub async fn run(command: Roles, handle: &Handle, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Roles::Create(arguments) => create::run(arguments, handle, emitter).await,
        Roles::Delete(arguments) => delete::run(arguments, handle, emitter).await,
        Roles::Edit(arguments) => edit::run(arguments, handle, emitter).await,
        Roles::Get(arguments) => get::run(arguments, handle, emitter).await,
        Roles::List(arguments) => list::run(arguments, handle, emitter).await,
        Roles::Tag(arguments) => tag::run(arguments, handle, emitter).await,
        Roles::Untag(arguments) => untag::run(arguments, handle, emitter).await,
    }
}
