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
    /// Set one place in the document, or delete it: the place a jq
    /// path expression names, the value as YAML. `edit SEL VALUE` is
    /// the program `(SEL) = $value`, `edit SEL --delete` the program
    /// `del(SEL)`, written only once the result passes the document's
    /// own type.
    Edit {
        /// The place, as a jq path expression, with its leading dot:
        /// `.port`, `.volumes.stores[0].capacity`, `.daemons[].default`,
        /// `.volumes.stores[] | select(.path == "/mnt/a") | .capacity`,
        /// `.volumes.stores[length]` to append. An object missing on
        /// the way is made, as jq makes it; an expression that names
        /// no place is jq's own refusal.
        #[arg(value_name = "SELECTOR")]
        selector: String,
        /// The value, as YAML, so it types without quoting: `15000` a
        /// number, `true` a bool, `root` a string, `"15000"` a string
        /// meant as one, `{kind: remote, url: "…"}` an object. JSON is
        /// YAML. Required unless `--delete`.
        #[arg(
            value_name = "VALUE",
            required_unless_present = "delete",
            conflicts_with = "delete"
        )]
        value: Option<String>,
        /// Delete what the selector names, instead of setting it.
        #[arg(long)]
        delete: bool,
    },
}
