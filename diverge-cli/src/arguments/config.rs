//! What `diverge config` takes: which document, and what to do to it.

use clap::{Args, Subcommand};

/// The `config` command: one of the four documents in the root, and
/// an action on it.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct Config {
    /// The document.
    #[command(subcommand)]
    pub document: Document,
}

/// The four documents in the root, each with its action.
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
    /// The viewer's `viewer.yaml`.
    Viewer {
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

/// What is done to a document: see [`run`](crate::cli::config::run)
/// for each in full. The document is read as its program
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
    /// Change the document by a JSON Patch: a list of operations, each
    /// on one place the document has — `add`, `remove`, `replace`,
    /// `move`, `copy`, `test`, RFC 6902, with JSON Pointer paths —
    /// applied in order, whole or not at all, and written only once
    /// the result passes the document's own type. The one way a record
    /// of the daemon's is edited, and the one way a document is.
    Patch {
        /// The patch, as YAML, JSON being YAML:
        /// `[{op: replace, path: /port, value: 15000}]`,
        /// `[{op: add, path: /volumes/stores/-, value: {path: /mnt/b, capacity: 1000000}}]`,
        /// `[{op: remove, path: /volumes/stores/0}]`. Every operation
        /// is as `diverge_sdk::daemon::patch::Operation` states it.
        #[arg(value_name = "PATCH")]
        patch: String,
    },
}
