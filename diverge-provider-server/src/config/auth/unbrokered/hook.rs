//! A hook that judges the credential.

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

/// A hook that judges the credential itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hook {
    /// The hook, by name: the folder `hooks/<name>/` of the
    /// provider's directory, run as [`hook`](crate::hook) provides.
    /// It reads a [`HookInput`] and writes a [`HookOutput`].
    pub authorize_hook: String,
}

/// What the hook receives on stdin, as one line of JSON.
///
/// ```json
/// {"credential": "5f1c…", "address": "203.0.113.7"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookInput {
    /// The credential the peer presented, as it was presented.
    pub credential: String,
    /// The peer's address, as the OS reported it: `203.0.113.7`, or
    /// `2001:db8::7`.
    pub address: IpAddr,
}

/// What the hook writes to stdout, as one JSON document, on exit `0`.
///
/// Anything else — a non-zero exit, a stdout that is not this — is a
/// [`hook::Error`](crate::hook::Error): the hook has not answered, and
/// the credential is refused.
///
/// ```json
/// {"authorized": true, "identity": "acme"}
/// ```
///
/// ```json
/// {"authorized": false}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookOutput {
    /// `true`, the credential is accepted; `false`, it is refused, and
    /// nothing of that reaches the peer.
    pub authorized: bool,
    /// Who presented it: the string every handler and every capability
    /// receives as the client's identity. Read only when `authorized`
    /// is `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
}
