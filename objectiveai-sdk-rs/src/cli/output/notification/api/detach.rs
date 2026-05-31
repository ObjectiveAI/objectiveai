use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Emitted by the parent process during `--detach` once the child has
/// been spawned. Replaces the prior `println!("PID: {pid}")`.
///
/// Wire: `{"type":"notification","pid":12345}`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(rename = "cli.output.notification.api.Detached")]
pub struct Detached {
    pub pid: u32,
}
