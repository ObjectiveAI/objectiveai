//! Who a run is, as the daemon attests it under `_meta`.

use diverge_sdk::daemon::key;
use diverge_sdk::shared::containers::request::Image;
use diverge_sdk::shared::mcp::Who;

/// The one agent or the one tool a run is: its key, and its image
/// when the daemon knows it. What
/// [`attest`](diverge_sdk::shared::mcp::attest) is told on every MCP
/// call the run sends outward.
#[derive(Debug, Clone)]
pub enum Caller {
    /// An agent, run from its template: always imaged.
    Agent { key: key::Agent, image: Image },
    /// A tool, run from its template or joined through its provider:
    /// imaged only when run.
    Tool { key: key::Tool, image: Option<Image> },
}

impl Caller {
    /// The key, as the attestation names it.
    pub fn who(&self) -> Who<'_> {
        match self {
            Caller::Agent { key, .. } => Who::Agent(key),
            Caller::Tool { key, .. } => Who::Tool(key),
        }
    }

    /// The image, when the daemon knows it.
    pub fn image(&self) -> Option<&Image> {
        match self {
            Caller::Agent { image, .. } => Some(image),
            Caller::Tool { image, .. } => image.as_ref(),
        }
    }
}
