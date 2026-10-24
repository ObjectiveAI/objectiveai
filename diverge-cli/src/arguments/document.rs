//! What `diverge viewer` and `diverge cli` take.

use clap::Subcommand;

use super::Action;

/// A program the CLI does not start — the viewer, or the CLI itself —
/// whose document in the root is worked on here.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Document {
    /// Read, rewrite or edit the program's document in the root:
    /// `viewer.yaml` or `cli.yaml`.
    #[command(subcommand)]
    Config(Action),
}
