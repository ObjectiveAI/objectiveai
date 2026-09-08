//! GCP Vertex AI.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// GCP Vertex AI — a service account, not an API key.
///
/// APPLICATION: the harness writes
/// [`service_account_json`](Self::service_account_json) to a file
/// and sets `VERTEX_CREDENTIALS_PATH` to it in the gateway's process
/// environment before Hermes starts; access tokens are minted from
/// it per call. `VERTEX_PROJECT_ID` and `VERTEX_REGION` are set from
/// the optional fields when present. Ambient application-default
/// credentials are deliberately absent — those are facts about a
/// machine, and a request has no machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `vertex`.
    pub provider: Vertex,
    /// The service-account key document, verbatim JSON.
    pub service_account_json: String,
    /// The project, applied as `VERTEX_PROJECT_ID`; absent = the
    /// project named inside the service-account document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// The region, applied as `VERTEX_REGION`; absent = Hermes's own
    /// default (`global`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Vertex {
    #[default]
    Vertex,
}
