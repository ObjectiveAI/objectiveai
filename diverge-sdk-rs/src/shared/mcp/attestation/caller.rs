//! Who is calling, on a request.

use serde::{Deserialize, Serialize};

use super::Agent;
use crate::shared::Identity;

/// The caller, as the daemon attests it on the `_meta` of the four
/// requests a tool receives — `list_tools`, `list_resources`,
/// `call_tool`, `read_resource`. Tagged `kind`, two kinds. From
/// inside the harness, an [`Agent`] of the daemon that runs the tool,
/// whose container sent the call: `kind` `agent`, with its image and
/// its key. From outside, a client of that daemon reaching the tool
/// over a connect: `kind` `client`, the identity the daemon
/// established at the handshake, and nothing more — not the agent the
/// client may say is calling, not its image, since nothing a caller
/// says of itself is trusted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Caller {
    /// An agent of this daemon's, calling from its container:
    /// `{"kind":"agent","image":{…},"template":…,"index":…,"name":…}`.
    Agent(Agent),
    /// A client of this daemon's, calling over a connect:
    /// `{"kind":"client","identity":{"issuer":…,"subject":…}}`.
    Client {
        /// The account the connection is: the identity its token
        /// established — as every `Creator::Client` names the same
        /// client.
        identity: Identity,
    },
}
