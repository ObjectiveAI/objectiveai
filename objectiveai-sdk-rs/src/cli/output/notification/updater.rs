use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire shapes emitted by the cli's `update` subcommand (and by
/// programmatic callers that invoke the cli's updater). Every skip
/// path AND every active step emits one of these.
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
    /// Refused to proceed with the update — the `reason` carries why.
    /// No binaries were modified.
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
    /// One binary's swap completed. Emitted once per package the
    /// updater touched (cli, api, viewer, mcp).
    #[schemars(title = "Installed")]
    Installed {
        current_version: String,
        remote_version: String,
    },
}

/// Reasons the updater can skip a run *and emit a notification about it*.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "cli.output.notification.SkipReason")]
pub enum SkipReason {
    /// Host OS/arch combination doesn't have a release asset.
    #[schemars(title = "UnsupportedPlatform")]
    UnsupportedPlatform,
    /// Binary is running out of a `target*/` directory — looks like a
    /// dev build (`cargo run`), not an installed binary.
    #[schemars(title = "DevTree")]
    DevTree,
    /// The latest release is missing one or more of the four expected
    /// assets for this host triple. No partial updates: refuse the
    /// whole run.
    #[schemars(title = "IncompleteRelease")]
    IncompleteRelease,
}
