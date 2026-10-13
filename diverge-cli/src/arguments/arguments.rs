//! What `diverge` takes.

use std::path::PathBuf;

use clap::Parser;

use super::Command;

/// The command line: where the configuration is, and what to do.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(name = "diverge", version, about = "The Diverge daemon's client")]
pub struct Arguments {
    /// The root: the directory `cli.json` is read from. Absent,
    /// `DIVERGE_CONFIG`, else `.diverge` in the working directory
    /// where it exists, else `.diverge` under the home directory.
    #[arg(long, value_name = "DIR")]
    pub config: Option<PathBuf>,
    /// What to do. Absent, nothing: the configuration is resolved and
    /// the program ends.
    #[command(subcommand)]
    pub command: Option<Command>,
}
