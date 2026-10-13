//! What `diverge` takes.

use std::path::PathBuf;

use clap::Parser;

use super::Command;

/// The command line: where the configuration is, which daemon and
/// which account of it to be, and what to do.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(name = "diverge", version, about = "The Diverge daemon's client")]
pub struct Arguments {
    /// The root: the directory `cli.json` is read from. Absent,
    /// `DIVERGE_CONFIG`, else `.diverge` in the working directory
    /// where it exists, else `.diverge` under the home directory.
    #[arg(long, value_name = "DIR")]
    pub config: Option<PathBuf>,
    /// The daemon to dial, by its name in the document. Absent, the
    /// document's default.
    #[arg(long, value_name = "NAME")]
    pub daemon: Option<String>,
    /// The account to dial as, by its name under the daemon. Absent,
    /// the daemon's default.
    #[arg(long, value_name = "NAME")]
    pub account: Option<String>,
    /// What to do. Absent, nothing: the configuration is resolved and
    /// the program ends.
    #[command(subcommand)]
    pub command: Option<Command>,
}
