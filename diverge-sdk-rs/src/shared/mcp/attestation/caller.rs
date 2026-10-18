//! Who is calling, on a request.

use serde::{Deserialize, Serialize};

use super::Agent;

/// The caller, as the daemon attests it on the `_meta` of the four
/// requests a tool receives — `list_tools`, `list_resources`,
/// `call_tool`, `read_resource`. Tagged `kind`, two kinds. From
/// inside the harness, an [`Agent`] of the daemon that runs the tool,
/// whose container sent the call: `kind` `agent`, with its image and
/// its key. From outside, a client of that daemon reaching the tool
/// over a connect: `kind` `unbrokered`, the identity the daemon
/// authorized the connection as, and nothing more — not the agent
/// the client may say is calling, not its image, since nothing a
/// caller says of itself is trusted, only what the daemon
/// established at the handshake. The kind is named after the auth
/// the connection came in on, so a brokered auth brings a brokered
/// kind beside this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Caller {
    /// An agent of this daemon's, calling from its container:
    /// `{"kind":"agent","image":{…},"template":…,"index":…,"name":…}`.
    Agent(Agent),
    /// A client of this daemon's, calling over a connect, as the
    /// daemon authorized it unbrokered:
    /// `{"kind":"unbrokered","identity":…}`.
    Unbrokered {
        /// What the daemon serves the connection as: the account's
        /// name when it has one, else its credential's identity — as
        /// every `Creator::Client` names the same client.
        identity: String,
    },
}
