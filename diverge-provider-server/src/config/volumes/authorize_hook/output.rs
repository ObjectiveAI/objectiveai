//! The document the hook writes.

use serde::{Deserialize, Serialize};

/// What the hook writes to stdout, as one JSON document, on exit `0`.
///
/// ```json
/// {"authorized": true}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    /// `true`, the volume is listed to the identity and may be
    /// mounted by it; `false`, it is not, and the identity is not
    /// told that it exists.
    pub authorized: bool,
}
