//! How a tool made from the template is kept running.

use serde::{Deserialize, Serialize};

/// Transient or a service: the `mode` a tool template says beside its
/// `type`, which every tool made from it runs by. Snake case on the
/// wire: `"transient"`, `"service"`; absent is transient.
///
/// # A transient tool runs while it is used
///
/// Its container is started for the first thing that uses it — an
/// agent it is attached to beginning a loop, a connect scope opened
/// on it, a file operation — and stopped when the last leaves. A
/// connected tool, another daemon's, is whatever that daemon's
/// template says.
///
/// # A service is kept while anything holds it
///
/// Its container is kept running for as long as any agent is attached
/// to it or any connect scope is open on it — started at the attach
/// or the connect that first holds it, started again whenever its run
/// ends, and started at every boot of the daemon's while an agent is
/// attached — and once nothing holds it its idle clock runs, and
/// `idle_seconds` after the last hold left with none since it is
/// retired: its run stopped, its record kept, to be kept up again by
/// the next attach or connect. This is the service a dependency
/// template declares, for a tool a caller makes by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Run while used, stopped when the last user leaves: the default.
    #[default]
    Transient,
    /// Kept running while any agent is attached or any connect scope
    /// is open, retired `idle_seconds` after the last leaves.
    Service,
}
