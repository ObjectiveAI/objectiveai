//! AWS Bedrock.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// AWS Bedrock — AWS credentials, not a provider key.
///
/// APPLICATION: the harness sets the AWS SDK's own env vars in the
/// gateway's process environment before Hermes starts — see
/// [`Auth`] for which — plus `AWS_REGION` to
/// [`region`](Self::region) when present. These must be REAL
/// process env: the botocore chain underneath never reads Hermes's
/// dotenv.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `bedrock`.
    pub provider: Bedrock,
    /// The credential itself. See [`Auth`].
    pub auth: Auth,
    /// The region, applied as `AWS_REGION`; absent = Hermes's own
    /// default (`us-east-1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// One of AWS's two static credential shapes. Untagged — the field
/// names are disjoint, so the shape is its own discriminator.
///
/// The rest of the botocore chain (profiles, container/IMDS roles,
/// web identity) is deliberately absent: those are facts about a
/// machine, and a request has no machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Auth {
    /// A Bedrock API key, applied as `AWS_BEARER_TOKEN_BEDROCK` —
    /// the first credential the chain checks.
    BearerToken {
        /// The key.
        bearer_token: String,
    },
    /// Static SigV4 credentials, applied as `AWS_ACCESS_KEY_ID`,
    /// `AWS_SECRET_ACCESS_KEY` and, when present,
    /// `AWS_SESSION_TOKEN`.
    AccessKey {
        /// The access key id.
        access_key_id: String,
        /// The secret access key.
        secret_access_key: String,
        /// The session token, for temporary credentials.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        session_token: Option<String>,
    },
}

/// [`Provider`]'s discriminator.
///
/// One variant, and the reason the [union](super::Provider) can be
/// untagged: no other provider's arguments can produce this value.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Bedrock {
    #[default]
    Bedrock,
}
