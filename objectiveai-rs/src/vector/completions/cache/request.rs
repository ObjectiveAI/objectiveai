use crate::chat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub enum CacheVoteRequest<'a> {
    Ref(CacheVoteRequestRef<'a>),
    Owned(CacheVoteRequestOwned),
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

#[derive(Debug, Clone, Serialize)]
pub struct CacheVoteRequestRef<'a> {
    pub model: &'a chat::completions::request::Model,
    pub models: Option<&'a [chat::completions::request::Model]>,
    pub messages: &'a [chat::completions::request::Message],
    pub tools: Option<&'a [chat::completions::request::Tool]>,
    pub responses: &'a [chat::completions::request::RichContent],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheVoteRequestOwned {
    pub model: chat::completions::request::Model,
    pub models: Option<Vec<chat::completions::request::Model>>,
    pub messages: Vec<chat::completions::request::Message>,
    pub tools: Option<Vec<chat::completions::request::Tool>>,
    pub responses: Vec<chat::completions::request::RichContent>,
}
