//! `diverge run agents templates …`.
//!
//! The requests over agent templates. One file per request, one folder
//! per family within: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod tag;
pub mod untag;

/// The requests over agent templates.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Templates {
    /// Making a template.
    Create(create::Arguments),
    /// Deleting a template by id.
    Delete(delete::Arguments),
    /// Getting one template.
    Get(get::Arguments),
    /// Listing a caller's templates.
    List(list::Arguments),
    /// Putting tags on a template.
    Tag(tag::Arguments),
    /// Taking tags off a template.
    Untag(untag::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(
    command: Templates,
    target: &Target<'_>,
    emitter: &mut Emitter,
) -> Result<(), Error> {
    match command {
        Templates::Create(arguments) => create::run(arguments, target, emitter).await,
        Templates::Delete(arguments) => delete::run(arguments, target, emitter).await,
        Templates::Get(arguments) => get::run(arguments, target, emitter).await,
        Templates::List(arguments) => list::run(arguments, target, emitter).await,
        Templates::Tag(arguments) => tag::run(arguments, target, emitter).await,
        Templates::Untag(arguments) => untag::run(arguments, target, emitter).await,
    }
}
