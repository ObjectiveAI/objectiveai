//! The line the hook reads.

use serde::{Deserialize, Serialize};

/// What the hook receives on stdin, as one line of JSON.
///
/// ```json
/// {"identity": "acme", "volume": "datasets"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// The identity asking: the string the authorizer answered when
    /// the client connected.
    pub identity: String,
    /// The fixed volume's `name`, as the configuration wrote it.
    pub volume: String,
}
