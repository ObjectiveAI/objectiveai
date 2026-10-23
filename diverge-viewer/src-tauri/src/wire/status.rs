//! How the link stands, as the page is told.

use serde::Serialize;
use ts_rs::TS;

/// How the link to the daemon stands: not yet dialled, being dialled,
/// up, or down with the reason. The page reads it on the settings
/// screen and decides from it whether to list anything.
#[derive(Serialize, TS, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
#[ts(export, export_to = "../../src/bindings/")]
pub enum WireStatus {
    /// Nothing has used the link yet.
    Idle,
    /// A use is dialling now.
    Dialling,
    /// The connection is up: the last dial succeeded and no send has
    /// found it gone since.
    Connected,
    /// The last dial failed, or a send found the connection gone; the
    /// next use dials again.
    Failed { message: String },
}
