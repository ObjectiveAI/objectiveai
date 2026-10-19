//! `diverge run providers daemons accounts …`.
//!
//! The requests over the accounts on daemon records. One file per
//! request, one folder per family within: the tree of files is the tree
//! of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod add;
pub mod delete;
pub mod edit;

/// The requests over the accounts on daemon records.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Accounts {
    /// Adding an account to a daemon record.
    Add(add::Arguments),
    /// Forgetting an account on a daemon record.
    Delete(delete::Arguments),
    /// Replacing an account's mode on a daemon record.
    Edit(edit::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Accounts,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Accounts::Add(arguments) => add::run(arguments, target, emitter).await,
        Accounts::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Accounts::Edit(arguments) => edit::run(arguments, target, emitter).await,
    }
}
