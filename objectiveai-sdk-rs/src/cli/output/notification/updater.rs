use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shapes emitted by `crate::updater::maybe_auto_update`. Every
/// non-cooldown skip path AND every active step emits one of these.
/// The ONLY silent path is the 2-hour marker cooldown — when the marker
/// says we already checked within the time frame, the updater exits
/// without emission.
///
/// Errors are emitted as `super::super::Output::Error` (level=warn,
/// fatal=false), not as a variant here.
///
/// Wire (in combination with `super::super::Output::Notification` +
/// `super::Notification`'s `value` wrapper):
///   `{"type":"notification","value":{"event":"checking",...}}`.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(tag = "event", rename_all = "snake_case")]
#[schemars(rename = "cli.output.notification.Updater")]
pub enum Updater {
    /// Skipped this run for a non-cooldown reason. (Cooldown is silent.)
    #[schemars(title = "Skipped")]
    Skipped {
        reason: SkipReason,
    },
    /// All gates passed; about to call
    /// `GET /repos/ObjectiveAI/objectiveai/releases/latest`. The very
    /// first emitted line of any active update run.
    #[schemars(title = "Checking")]
    Checking {
        asset_name: String,
        current_version: String,
    },
    /// GitHub returned the latest release tag and it's ≤ current.
    /// Terminal — no more events follow.
    #[schemars(title = "UpToDate")]
    UpToDate {
        current_version: String,
        remote_version: String,
    },
    /// Found a newer release with our asset attached; about to download.
    #[schemars(title = "Found")]
    Found {
        current_version: String,
        remote_version: String,
        asset_name: String,
        url: String,
    },
    /// Swap complete; the next line of output will come from the
    /// re-exec'd new binary, not this one. Terminal for the current
    /// process.
    #[schemars(title = "Installed")]
    Installed {
        current_version: String,
        remote_version: String,
    },
}

/// Reasons the updater can skip a run *and emit a notification about it*.
/// The 2-hour marker cooldown is NOT included here because cooldown
/// skips are silent (no notification emitted).
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.output.notification.SkipReason")]
pub enum SkipReason {
    /// Host OS/arch combination doesn't have a release asset.
    #[schemars(title = "UnsupportedPlatform")]
    UnsupportedPlatform,
    /// `OBJECTIVEAI_SKIP_UPDATE` env var is set. The updater respects
    /// this so re-exec'd children don't loop, and so users can disable
    /// auto-update by setting it.
    #[schemars(title = "OptedOut")]
    OptedOut,
    /// Binary is running out of a `target*/` directory — looks like a
    /// dev build (`cargo run`), not an installed binary.
    #[schemars(title = "DevTree")]
    DevTree,
}
