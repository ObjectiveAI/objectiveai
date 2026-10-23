//! What `diverge keyring` takes.

use clap::Subcommand;

/// The host's keyring at the root, `keyring.db`, worked on here and
/// nowhere else: no daemon is dialled, and no command prints a key.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Keyring {
    /// Every entry, by name, with when its key was made: one JSON
    /// line each, never a key.
    List,
    /// Mint a new key as an entry. A name already held is refused.
    Mint {
        /// The entry's name.
        name: String,
    },
    /// Keep a key someone gave you as an entry, read from stdin to
    /// its end, the trailing line end dropped. A name already held is
    /// refused.
    Set {
        /// The entry's name.
        name: String,
    },
    /// Take an entry and its key out.
    Remove {
        /// The entry's name.
        name: String,
    },
}
