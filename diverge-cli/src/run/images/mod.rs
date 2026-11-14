//! `diverge run images …`.
//!
//! The requests over images: named by digest, never held. One file per
//! request: the tree of files is the tree of subcommands.

use clap::Subcommand;

use crate::run::{Emitter, Error, Target};

pub mod build;
pub mod read;

/// The requests over images.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Images {
    /// Build an image on a provider, from a Containerfile.
    Build(build::Arguments),
    /// Whether the daemon can supply an image, through a provider or a
    /// remote daemon its grants reach, and, where it can, one ask of
    /// it: its DIVERGE.md, a manifest, a blob, or the proof.
    Read(read::Arguments),
}

/// The request put, by its subcommand.
pub async fn run(command: Images, target: &Target<'_>, emitter: &mut Emitter) -> Result<(), Error> {
    match command {
        Images::Build(arguments) => build::run(arguments, target, emitter).await,
        Images::Read(arguments) => read::run(arguments, target, emitter).await,
    }
}
