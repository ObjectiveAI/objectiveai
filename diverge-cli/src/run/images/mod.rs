//! `diverge run images …`.
//!
//! The requests over images: named by digest, never held. One file per
//! request: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod info;

/// The requests over images.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Images {
    /// Reading an image's DIVERGE.md, from a container of it running or
    /// from a provider that can obtain it.
    Info(info::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(command: Images, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Images::Info(arguments) => info::run(arguments, target, emitter).await,
    }
}
