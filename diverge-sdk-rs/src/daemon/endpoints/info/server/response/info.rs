//! What the daemon says it is.

use serde::{Deserialize, Serialize};

/// What the daemon says it is: its version.
///
/// ```json
/// {"version":"2.3.0"}
/// ```
///
/// A member this version does not know is ignored, so a newer daemon's
/// answer is read rather than refused.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Info {
    /// The daemon's version, `MAJOR.MINOR.PATCH`: the revision of the
    /// protocol it speaks.
    pub version: String,
}
