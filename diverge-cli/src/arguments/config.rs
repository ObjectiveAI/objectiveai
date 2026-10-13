//! What `diverge config` takes: which document, and what to do to it.

use clap::{Args, Subcommand};

/// The `config` command: one of the three documents in the root, and
/// an action on it.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Config {
    /// The document.
    #[command(subcommand)]
    pub document: Document,
}

/// The three documents in the root, each with its action.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Document {
    /// The daemon's `daemon.yaml`.
    Daemon {
        /// What to do to it.
        #[command(subcommand)]
        action: Action,
    },
    /// The provider's `provider.yaml`.
    Provider {
        /// What to do to it.
        #[command(subcommand)]
        action: Action,
    },
    /// The CLI's own `cli.yaml`.
    Cli {
        /// What to do to it.
        #[command(subcommand)]
        action: Action,
    },
}

/// What is done to a document. The document is read as its program
/// reads it — the built-in defaults where there is no file — and
/// handled as JSON on this side of the command line, the file's YAML
/// notwithstanding.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Action {
    /// Print the document, narrowed by a jq filter, as JSON: every
    /// value the filter yields, one line each.
    Get {
        /// The jq filter. Absent, `.`, the whole document.
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },
    /// Rewrite the document by a jq program: the one value it yields
    /// is the whole new document, written only once it passes the
    /// document's own type.
    Set {
        /// The jq program, run with the document as its input. It
        /// must yield exactly one value.
        #[arg(value_name = "FILTER")]
        filter: String,
    },
    /// Open the document in `$VISUAL`, else `$EDITOR`, and write back
    /// what comes out, once it passes the document's own type.
    Edit,
}
