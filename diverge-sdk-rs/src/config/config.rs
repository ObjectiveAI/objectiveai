//! The document that is `config.yaml`.

use serde::{Deserialize, Serialize};

use super::{daemon, provider};

/// The whole of `config.yaml`: one block per program.
///
/// An absent file is this type's [`Default`], every block its own.
/// A block left out of the file is its default too, and a key that
/// is not a block is an error.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// What `diverge-provider` is told: see [`provider::Config`].
    pub provider: provider::Config,
    /// What `diverge-daemon` is told, and what `diverge-postgres`
    /// takes from it: see [`daemon::Config`].
    pub daemon: daemon::Config,
}
