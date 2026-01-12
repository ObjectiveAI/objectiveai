use crate::prefixed_uuid::PrefixedUuid;
use serde::{Deserialize, Serialize};

pub type ApiKey = PrefixedUuid<'a', 'p', 'k'>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyWithMetadata {
    pub api_key: ApiKey,
    pub created: chrono::DateTime<chrono::Utc>,
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    pub disabled: Option<chrono::DateTime<chrono::Utc>>,
    pub name: String,
    pub description: Option<String>,
}
