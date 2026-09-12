//! The line the hook reads.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// What the hook receives on stdin, as one line of JSON.
///
/// ```json
/// {"credential": "5f1c…", "address": "203.0.113.7"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    /// The credential the peer presented, as it was presented.
    pub credential: String,
    /// The peer's address, as the OS reported it: `203.0.113.7`, or
    /// `2001:db8::7`.
    pub address: IpAddr,
}
