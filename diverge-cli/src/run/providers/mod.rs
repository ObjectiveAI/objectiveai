//! `diverge run providers …`.
//!
//! The requests over providers — the records of the providers the
//! daemon works with. One file per
//! request, one folder per family within: the tree of files is the tree
//! of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod add;
pub mod delete;
pub mod disable;
pub mod discover;
pub mod edit;
pub mod enable;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

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
    /// Adding a provider's record: dialled at once when it is outgoing
    /// and enabled.
    Add(add::Arguments),
    /// Editing a provider's record: a change to its direction or its
    /// discover flag closes its connection.
    Edit(edit::Arguments),
    /// Deleting a disabled provider's record that nothing is pinned to.
    Delete(delete::Arguments),
    /// Enabling a provider: dialled again when outgoing, admitted again
    /// when incoming.
    Enable(enable::Arguments),
    /// Disabling a provider: its dial stopped, its connection closed,
    /// its runs taken down.
    Disable(disable::Arguments),
    /// Tagging a provider's record.
    Tag(tag::Arguments),
    /// Untagging a provider's record.
    Untag(untag::Arguments),
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
        Providers::Add(arguments) => add::run(arguments, target, emitter).await,
        Providers::Edit(arguments) => edit::run(arguments, target, emitter).await,
        Providers::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Providers::Enable(arguments) => enable::run(arguments, target, emitter).await,
        Providers::Disable(arguments) => disable::run(arguments, target, emitter).await,
        Providers::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Providers::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
