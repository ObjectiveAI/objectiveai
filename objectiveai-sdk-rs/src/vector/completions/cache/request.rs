use crate::agent;
use objectiveai_sdk_macros::schema_override;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request body for retrieving completion votes by vector completion ID.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.cache.GetCompletionVotesRequest")]
pub struct GetCompletionVotesRequest {
    /// The vector completion ID.
    pub id: String,
}

#[schema_override(RefOwnedEnum)]
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum CacheVoteRequest<'a> {
    Ref(CacheVoteRequestRef<'a>),
    Owned(CacheVoteRequestOwned),
}

impl schemars::JsonSchema for CacheVoteRequest<'static> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        CacheVoteRequestOwned::schema_name()
    }
    fn json_schema(
        generator: &mut schemars::SchemaGenerator,
    ) -> schemars::Schema {
        CacheVoteRequestOwned::json_schema(generator)
    }
}

impl<'de> serde::de::Deserialize<'de> for CacheVoteRequest<'static> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let owned = CacheVoteRequestOwned::deserialize(deserializer)?;
        Ok(CacheVoteRequest::Owned(owned))
    }
}

#[schema_override(Ref)]
#[derive(Debug, Clone, Serialize)]
pub struct CacheVoteRequestRef<'a> {
    pub agent: &'a agent::InlineAgentBaseWithFallbacksOrRemote,
    pub messages: &'a [agent::completions::message::Message],
    pub responses: &'a [agent::completions::message::RichContent],
}

#[schema_override(Owned)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "vector.completions.cache.CacheVoteRequest")]
pub struct CacheVoteRequestOwned {
    pub agent: agent::InlineAgentBaseWithFallbacksOrRemote,
    pub messages: Vec<agent::completions::message::Message>,
    pub responses: Vec<agent::completions::message::RichContent>,
}
