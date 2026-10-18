//! AWS Bedrock.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// AWS Bedrock — AWS credentials, not a provider key.
///
/// APPLICATION: the harness sets the AWS SDK's own env vars in the
/// gateway's process environment before Hermes starts, each to its
/// secret's value — see [`Auth`] for which — plus `AWS_REGION` to
/// [`region`](Self::region) when present. These must be REAL
/// process env: the botocore chain underneath never reads Hermes's
/// dotenv.
///
/// SECRET: the set [`auth`](Self::auth) fixes, every one fixed —
/// `AWS_BEARER_TOKEN_BEDROCK` for `bearer-token`;
/// `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` for `access-key`;
/// those two and `AWS_SESSION_TOKEN` for `session-token`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Provider {
    /// The discriminator. Always `bedrock`.
    pub provider: Bedrock,
    /// Which credential shape the secrets carry. See [`Auth`].
    pub auth: Auth,
    /// The region, applied as `AWS_REGION`; absent = Hermes's own
    /// default (`us-east-1`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// One of AWS's static credential shapes, by name — which fixes the
/// secrets the provider declares. Kebab-case on the wire.
///
/// The rest of the botocore chain (profiles, container/IMDS roles,
/// web identity) is deliberately absent: those are facts about a
/// machine, and a request has no machine.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Auth {
    /// A Bedrock API key — the first credential the chain checks.
    /// Declares `AWS_BEARER_TOKEN_BEDROCK`.
    BearerToken,
    /// Static SigV4 credentials. Declares `AWS_ACCESS_KEY_ID` and
    /// `AWS_SECRET_ACCESS_KEY`.
    AccessKey,
    /// Temporary SigV4 credentials. Declares `AWS_ACCESS_KEY_ID`,
    /// `AWS_SECRET_ACCESS_KEY` and `AWS_SESSION_TOKEN`.
    SessionToken,
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
