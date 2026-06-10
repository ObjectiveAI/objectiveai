//! `PluginPath` — the `(owner, repository, version)` coordinate of the
//! plugin a command is running on behalf of.
//!
//! Set on [`crate::context::Context::plugin`] when the CLI executes a
//! command that originated from a plugin's bidirectional protocol. The
//! three parts are threaded to nested-command subprocesses via the
//! `OBJECTIVEAI_PLUGIN_OWNER` / `_REPOSITORY` / `_VERSION` env vars
//! (see [`crate::spawn::apply_config_env`]).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPath {
    pub owner: String,
    pub repository: String,
    pub version: String,
}

impl PluginPath {
    /// Assemble a `PluginPath` from three optional parts. `Some` only
    /// when all three are present; any missing part yields `None`.
    pub fn from_parts(
        owner: Option<String>,
        repository: Option<String>,
        version: Option<String>,
    ) -> Option<Self> {
        match (owner, repository, version) {
            (Some(owner), Some(repository), Some(version)) => Some(Self {
                owner,
                repository,
                version,
            }),
            _ => None,
        }
    }
}
