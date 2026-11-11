//! A tool call the run went down under, as the tool's log keeps it.

use serde::{Deserialize, Serialize};

/// A [`Call`](super::Call) still open when the tool's run went down:
/// its answer, kept by the daemon just before the run's `inactive` and
/// `undeployed`, whose `reason` says why — see
/// [`lifecycle`](crate::daemon::lifecycle). The caller was answered
/// with an error saying the call was interrupted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct CallInterrupted {
    /// The call's `logs_index`.
    pub call: u64,
}
